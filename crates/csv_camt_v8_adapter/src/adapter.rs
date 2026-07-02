use csv_camt_v8_domain::{BetragEntity, BuchungAggregat, BuchungsartEntity, WährungEntity};
use eigene_konten_adapter::Kontenregister;
use obfuskierung_adapter::Obfuskierer;

use crate::betrag::{währung_from_str, währung_normalized};
use crate::bridge;
use crate::buchung::Buchung;
use crate::fehler::ImportError;
use crate::port::{Umsatzquelle, Zeilenquelle};
use crate::zeile::Umsatzzeile;

/// Der CSV-CAMT-v8-Adapter: bindet eine [`Zeilenquelle`] (bereits geparste Zeilen)
/// an den Eigene-Konten-Slice und erfüllt so den [`Umsatzquelle`]-Port. Die
/// Obfuskierung geschieht beim Mappen über die Bridges — rohe Identifikatoren
/// erreichen die Domäne nie.
pub struct CsvCamtV8<Q: Zeilenquelle> {
    quelle: Q,
    konten: Kontenregister,
}

impl<Q: Zeilenquelle> CsvCamtV8<Q> {
    /// Baut den Adapter aus einer Zeilen-Quelle und dem geladenen Kontenregister.
    pub fn new(quelle: Q, konten: Kontenregister) -> Self {
        Self { quelle, konten }
    }
}

impl<Q: Zeilenquelle> Umsatzquelle for CsvCamtV8<Q> {
    fn buchungen(&self) -> Result<Vec<Buchung>, ImportError> {
        let zeilen = self.quelle.zeilen().map_err(ImportError::Quelle)?;
        buchungen_aus_zeilen(&zeilen, &self.konten)
    }
}

/// Der reine Kern: geparste Zeilen + Kontenregister → obfuskierte Buchungen. Zuerst
/// wird eine einheitliche Währung über die ganze Datei zugesichert, dann jede Zeile
/// obfuskierend gemappt. Die First-Seen-Vergabe (`KONTO_x`/`PERSON_x`) lebt genau
/// diesen Lauf — die laufübergreifende Persistenz ist vertagt (ADR-0006).
fn buchungen_aus_zeilen(
    zeilen: &[Umsatzzeile],
    konten: &Kontenregister,
) -> Result<Vec<Buchung>, ImportError> {
    let Some(währung) = datei_währung(zeilen)? else {
        return Ok(Vec::new());
    };
    let mut obfuskierer = Obfuskierer::new();
    zeilen
        .iter()
        .map(|zeile| zeile_zu_buchung(zeile, konten, währung, &mut obfuskierer))
        .collect()
}

/// Sichert **eine** Währung über die ganze Datei zu: die erste Zeile legt sie fest,
/// jede abweichende bricht laut ab. `Ok(None)`, wenn die Datei keine Datenzeile hat;
/// eine unbekannte Währung bricht ab, statt still falsch zu summieren.
fn datei_währung(zeilen: &[Umsatzzeile]) -> Result<Option<WährungEntity>, ImportError> {
    let mut codes = zeilen
        .iter()
        .map(|zeile| währung_normalized(&zeile.waehrung));
    let Some(first) = codes.next() else {
        return Ok(None);
    };
    if let Some(abweichend) = codes.find(|code| *code != first) {
        return Err(ImportError::MixedWährung(first, abweichend));
    }
    währung_from_str(&first)
        .map(Some)
        .ok_or(ImportError::UnknownWährung(first))
}

/// Mappt eine Zeile obfuskierend auf eine [`Buchung`]: die Bridges liefern die
/// obfuskierten Domänen-Werte, die Domäne deutet die Feld-Semantik (Betrag,
/// Buchungsart); daraus wird das Aggregat gebaut und auf das Ausgangs-DTO
/// projiziert. Die Währung ist die bereits als einheitlich geprüfte Datei-Währung.
fn zeile_zu_buchung(
    zeile: &Umsatzzeile,
    konten: &Kontenregister,
    währung: WährungEntity,
    obfuskierer: &mut Obfuskierer,
) -> Result<Buchung, ImportError> {
    let konto = bridge::obfuskiere_auftragskonto(&zeile.auftragskonto, konten, obfuskierer)?;
    let (richtung, betrag) = BetragEntity::aus_deutschem_betrag(&zeile.betrag, währung)
        .ok_or_else(|| ImportError::BetragInvalid(zeile.betrag.clone()))?;
    let empfänger = bridge::empfänger(&zeile.glaeubiger_id, &zeile.beguenstigter, obfuskierer)?;
    let gegenkonto = bridge::gegenkonto(&zeile.gegen_iban, konten);
    let verwendungszweck = bridge::verwendungszweck(&zeile.verwendungszweck, obfuskierer);
    let buchungsart = BuchungsartEntity::aus_buchungstext(&zeile.buchungstext);
    let aggregat = BuchungAggregat::new(
        konto,
        betrag,
        richtung,
        empfänger,
        gegenkonto,
        verwendungszweck,
        buchungsart,
    );
    Ok(Buchung::from_aggregat(&aggregat))
}

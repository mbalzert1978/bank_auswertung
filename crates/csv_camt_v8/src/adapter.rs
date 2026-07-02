use domain::{Buchung, Währung};
use eigene_konten::EigeneKonten;

use crate::fehler::ImportError;
use crate::feldparser::{währung_from_str, währung_normalized};
use crate::obfuskierung::Obfuskierung;
use crate::port::{CsvQuelle, Umsatzquelle};
use crate::row::RawRow;

/// Der CSV-CAMT-v8-Adapter: bindet eine [`CsvQuelle`] (rohe CSV) an die
/// Eigene-Konten-Config und erfüllt so den [`Umsatzquelle`]-Port. Die
/// Obfuskierung geschieht beim Mappen — siehe Crate-Doku.
pub struct CsvCamtV8<Q: CsvQuelle> {
    source: Q,
    konten: EigeneKonten,
}

impl<Q: CsvQuelle> CsvCamtV8<Q> {
    /// Baut den Adapter aus einer CSV-Quelle und der Eigene-Konten-Config.
    pub fn new(source: Q, accounts: EigeneKonten) -> Self {
        Self {
            source,
            konten: accounts,
        }
    }
}

impl<Q: CsvQuelle> Umsatzquelle for CsvCamtV8<Q> {
    fn buchungen(&self) -> Result<Vec<Buchung>, ImportError> {
        let content = self.source.read().map_err(ImportError::Source)?;
        buchungen_from_csv(&content, &self.konten)
    }
}

/// Der reine Kern: rohes CSV + Config → obfuskierte Buchungen. Zuerst wird eine
/// einheitliche Währung über die ganze Datei zugesichert, dann jede Zeile
/// obfuskierend gemappt; die First-Seen-Register (`KONTO_x`/`PERSON_x`) leben
/// genau diesen Lauf.
fn buchungen_from_csv(content: &str, accounts: &EigeneKonten) -> Result<Vec<Buchung>, ImportError> {
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(b';')
        .has_headers(true)
        .from_reader(content.as_bytes());
    let rows: Vec<RawRow> = reader.deserialize().collect::<Result<_, _>>()?;

    let Some(currency) = file_währung(&rows)? else {
        return Ok(Vec::new());
    };

    let mut obf = Obfuskierung::default();
    rows.iter()
        .map(|row| row.to_buchung(accounts, currency, &mut obf))
        .collect()
}

/// Sichert **eine** Währung über die ganze Datei zu: die erste Zeile legt sie
/// fest, jede abweichende bricht laut ab. `Ok(None)`, wenn die Datei keine
/// Datenzeile hat.
fn file_währung(rows: &[RawRow]) -> Result<Option<Währung>, ImportError> {
    let mut codes = rows.iter().map(|row| währung_normalized(row.währung_raw()));
    let Some(first) = codes.next() else {
        return Ok(None);
    };
    for code in codes {
        if code != first {
            return Err(ImportError::MixedWährung(first, code));
        }
    }
    währung_from_str(&first).map(Some)
}

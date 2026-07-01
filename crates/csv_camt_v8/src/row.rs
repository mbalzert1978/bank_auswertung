use domain::{Buchung, Empfänger, Gegenkonto, GläubigerId, ObfuskierterText, Währung};
use eigene_konten::{EigeneKonten, Iban, Kontozuordnung};
use serde::Deserialize;

use crate::fehler::ImportError;
use crate::feldparser::{betrag_parse, buchungsart_from_text};
use crate::obfuskierung::Obfuskierung;

/// Eine rohe CSV-Zeile: nur die Spalten, die der Adapter tatsächlich braucht,
/// über die Kopfzeilen-Namen zugeordnet (übrige Spalten wie `Buchungstag` oder
/// `Mandatsreferenz` werden ignoriert). Diese Rohform trägt **noch** die
/// strukturellen Identifikatoren (IBAN/BIC) — sie verlässt [`RawRow`] nie:
/// [`RawRow::to_buchung`] liefert ausschließlich die obfuskierte `Buchung`.
#[derive(Debug, Deserialize)]
pub(crate) struct RawRow {
    #[serde(rename = "Auftragskonto")]
    auftragskonto: String,
    #[serde(rename = "Buchungstext")]
    buchungstext: String,
    #[serde(rename = "Verwendungszweck")]
    verwendungszweck: String,
    #[serde(rename = "Glaeubiger ID")]
    glaeubiger_id: String,
    #[serde(rename = "Beguenstigter/Zahlungspflichtiger")]
    beguenstigter: String,
    #[serde(rename = "Kontonummer/IBAN")]
    gegen_iban: String,
    #[serde(rename = "Betrag")]
    betrag: String,
    #[serde(rename = "Waehrung")]
    waehrung: String,
}

impl RawRow {
    /// Der rohe Währungscode der Zeile (ungetrimmt) — die Normalisierung
    /// (trim + Großbuchstaben) übernimmt zentral
    /// [`crate::feldparser::währung_normalized`].
    pub(crate) fn währung_raw(&self) -> &str {
        &self.waehrung
    }

    /// Mappt die Zeile auf eine obfuskierte [`Buchung`]. Die Währung ist die
    /// bereits als einheitlich geprüfte Datei-Währung.
    pub(crate) fn to_buchung(
        &self,
        accounts: &EigeneKonten,
        currency: Währung,
        obf: &mut Obfuskierung,
    ) -> Result<Buchung, ImportError> {
        let account = obf.konto_id(&self.auftragskonto, accounts)?;
        let (direction, amount) = betrag_parse(&self.betrag, currency)?;
        let recipient = self.empfänger(obf)?;
        let counter_account = self.gegenkonto(accounts);
        let purpose = ObfuskierterText::new(&self.verwendungszweck);
        let booking_type = buchungsart_from_text(&self.buchungstext);
        Ok(Buchung::new(
            account,
            amount,
            direction,
            recipient,
            counter_account,
            purpose,
            booking_type,
        ))
    }

    /// Löst den Empfänger auf. Die Fail-safe-Weiche „Gläubiger-ID → Firma, sonst
    /// Person" liegt bewusst in der Domäne ([`Empfänger::resolve`]) — der Adapter
    /// liefert nur die Rohdaten und eine **lazy** Pseudonym-Quelle (First-Seen
    /// `PERSON_x`, nur im Person-Fall abgefragt, damit Firmen keinen Zähler
    /// verbrauchen). Ergibt die Auflösung `None` (weder Gläubiger-ID noch ein
    /// Name), ist die Gegenpartei nicht identifizierbar → `EmpfängerMissing`.
    fn empfänger(&self, obf: &mut Obfuskierung) -> Result<Empfänger, ImportError> {
        Empfänger::resolve(GläubigerId::new(&self.glaeubiger_id), || {
            obf.pseudonym(&self.beguenstigter)
        })
        .ok_or(ImportError::EmpfängerMissing)
    }

    /// Löst die Gegenseite über die Eigene-Konten-Config auf: eigene IBAN →
    /// `Eigen` (mit Rolle), sonst `Fremd`. Eine leere oder nicht als IBAN
    /// parsbare Gegenseite ist definitiv keines der eigenen Konten → `Fremd`.
    fn gegenkonto(&self, accounts: &EigeneKonten) -> Gegenkonto {
        match Iban::parse(&self.gegen_iban) {
            Ok(iban) => match accounts.lookup(&iban) {
                Kontozuordnung::Eigen(role) => Gegenkonto::Eigen(role),
                Kontozuordnung::Fremd => Gegenkonto::Fremd,
            },
            Err(_) => Gegenkonto::Fremd,
        }
    }
}

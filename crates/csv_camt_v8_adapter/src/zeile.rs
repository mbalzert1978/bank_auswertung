/// Eine bereits geparste CSV-Zeile als Eingangs-DTO des Seams: nur die Felder, die
/// der Adapter tatsächlich braucht (übrige CAMT-v8-Spalten wie `Buchungstag` oder
/// `Mandatsreferenz` interessieren nicht), als roher Text. **IO** füllt die Felder
/// aus den CSV-Spalten; der Adapter mappt die Zeile obfuskierend auf eine
/// [`Buchung`](crate::Buchung).
///
/// Datenschutz: trägt **noch** rohe strukturelle Identifikatoren (`auftragskonto`,
/// `gegen_iban`) und leitet daher **kein** `Serialize`/`Deserialize` ab
/// (compilergeprüft in `lib.rs`) — die Rohform verlässt die Maschine nie; sie wird
/// im Adapter zu obfuskierten Werten gemappt.
#[derive(Debug, Clone, Default)]
pub struct Umsatzzeile {
    /// Eigene IBAN, deren Umsatz die Zeile ist.
    pub auftragskonto: String,
    /// Der Buchungstext (die maschinelle Art).
    pub buchungstext: String,
    /// Der Verwendungszweck-Freitext.
    pub verwendungszweck: String,
    /// Die SEPA-Gläubiger-ID (kann leer sein).
    pub glaeubiger_id: String,
    /// Der Empfänger-/Zahlungspflichtigen-Name (Klartext).
    pub beguenstigter: String,
    /// Die Gegen-IBAN (kann leer/kein IBAN sein).
    pub gegen_iban: String,
    /// Der deutsche Geldbetrag mit Vorzeichen (`-1.234,56`).
    pub betrag: String,
    /// Der Währungscode (`EUR`).
    pub waehrung: String,
}

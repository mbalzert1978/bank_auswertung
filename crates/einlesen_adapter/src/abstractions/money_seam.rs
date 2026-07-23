/// Der öffentliche, mechanische Geld-Seam ([ADR-0008](../../../../docs/adr/0008-event-getriebene-pipeline-und-outbox.md)):
/// dekodiert das rohe Betragsfeld (deutsches Zahlenformat) zu seinen **unskalierten**
/// Bestandteilen — der Ziffernmenge, ihrer Nachkommastellen-Zahl und dem Vorzeichen —
/// **ohne die Währung zu kennen**. Die währungs-bewusste Skalierung auf die Minor Units
/// (ISO 4217) macht die `MoneyBridge` darüber, indem sie diesen Seam mit der vom
/// [`CurrencySeam`](crate::CurrencySeam) gelieferten Minor Unit paart. **Kein Domänentyp** — die
/// Bestandteile sind Primitive,
/// sodass IO diesen Seam ohne Domänen-Kenntnis erfüllt; **alle** Format-/Vorzeichen-Regeln
/// (Tausenderpunkt, Dezimalkomma, Nachkommastellen, Gruppierung, Erkennen/Abstreifen des
/// führenden Vorzeichens) liegen bei der Implementierung (io). Vorbild `ISftpPuller` der
/// C#-Referenz: ein öffentlicher, roh-typisierter Seam, den nur IO erfüllt.
pub trait MoneySeam {
    /// Dekodiert das rohe Betragsfeld zu seinen **unskalierten** Bestandteilen, oder `None`,
    /// wenn es kein gültiger Betrag ist — auch ein Feld **ohne** deutbares Vorzeichen ergibt
    /// `None` (die Domäne macht daraus `AmountInvalid`), nie ein still-positives Ergebnis.
    fn decode(&self, raw: &str) -> Option<DecodedAmount>;
}

/// Die **unskalierten** Bestandteile eines dekodierten Betrags — **adapter-eigen** (kein
/// Domänentyp): plain Felder, weil dieser Typ die Adapter-Grenze überquert (Seam-Rückgabewert)
/// und keine Domänen-Interna dorthin durchreichen soll. Der Wert ist `units / 10^decimal_places`;
/// die `MoneyBridge` skaliert ihn mit der Minor Unit der [`Currency`](einlesen_domain::Currency)
/// (deren Nachkommastellen der `CurrencySeam` liefert) auf die Minor Units der Währung.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecodedAmount {
    /// Die **vorzeichenlose** Ziffernmenge (Komma entfernt): der Wert ist `units / 10^decimal_places`.
    pub units: u64,
    /// Die Zahl der Nachkommastellen im Rohfeld (z. B. `1,234` → 3), aus der die Bridge auf die
    /// Minor Units der Währung skaliert.
    pub decimal_places: u32,
    /// Das erkannte (und aus den Ziffern abgestreifte) führende Vorzeichen.
    pub sign: Sign,
}

/// Das führende Vorzeichen des Betragsfelds — **zwei-wertig** (`Negative`/`Positive`).
/// Ein fehlendes Vorzeichen ist kein eigener Wert: der Seam lässt `decode` dann `None`
/// liefern (die Domäne sieht `AmountInvalid`), statt still auf positiv zu defaulten.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sign {
    Negative,
    Positive,
}

/// Der öffentliche, mechanische Geld-Seam ([ADR-0008](../../../../docs/adr/0008-event-getriebene-pipeline-und-outbox.md)):
/// dekodiert das rohe Betragsfeld (deutsches Zahlenformat) zu seinen Bestandteilen.
/// **Kein Domänentyp** — die Bestandteile sind Primitive, sodass IO diesen Seam ohne
/// Domänen-Kenntnis erfüllen kann; **alle** Format-/Vorzeichen-Regeln (Tausenderpunkt,
/// Dezimalkomma, Nachkommastellen, Gruppierung, Erkennen/Abstreifen des führenden
/// Vorzeichens) liegen bei der Implementierung (io, deferred). Vorbild `ISftpPuller` der
/// C#-Referenz: ein öffentlicher, roh-typisierter Seam, den nur IO erfüllt — die
/// `MoneyBridge` mappt ihn auf den internen Domänen-Port
/// `einlesen_domain::Money`, den der Handler verlangt.
pub trait MoneySeam {
    /// Dekodiert das rohe Betragsfeld zu seinen Bestandteilen, oder `None`, wenn es kein
    /// gültiger Betrag ist — auch ein Feld **ohne** deutbares Vorzeichen ergibt `None`
    /// (die Domäne macht daraus `AmountInvalid`), nie ein still-positives Ergebnis.
    fn decode(&self, raw: &str) -> Option<DecodedAmount>;
}

/// Die Bestandteile eines dekodierten Betrags — **adapter-eigen** (kein Domänentyp):
/// plain Felder, weil dieser Typ die Adapter-Grenze überquert (Seam-Rückgabewert) und
/// keine Domänen-Interna dorthin durchreichen soll.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecodedAmount {
    /// Die **vorzeichenlose** Menge in Cent.
    pub cent: u64,
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

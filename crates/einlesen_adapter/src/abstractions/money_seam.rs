/// Der öffentliche, mechanische Geld-Seam ([ADR-0008](../../../../docs/adr/0008-event-getriebene-pipeline-und-outbox.md)):
/// dekodiert das rohe Betragsfeld (deutsches Zahlenformat) zu seinen Bestandteilen.
/// **Kein Domänentyp** — die Bestandteile sind Primitive, sodass IO diesen Seam ohne
/// Domänen-Kenntnis erfüllen kann; die Format-Regeln (Tausenderpunkt, Dezimalkomma,
/// genau zwei Nachkommastellen, Gruppierungs-Prüfung, Erkennen/Abstreifen des führenden
/// Vorzeichens) liegen bei der Implementierung. Vorbild `ISftpPuller` der C#-Referenz:
/// ein öffentlicher, roh-typisierter Seam, den nur IO erfüllt — die
/// [`MoneyBridge`](crate::bridges::MoneyBridge) mappt ihn auf den internen Domänen-Port
/// `einlesen_domain::Money`, den der Handler verlangt.
pub trait MoneySeam {
    /// Dekodiert das rohe Betragsfeld zu seinen Bestandteilen, oder `None`, wenn es kein
    /// gültiger Betrag ist. Ein fehlendes Vorzeichen ist kein Dekodierfehler, sondern
    /// [`Sign::Missing`].
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

/// Das führende Vorzeichen des Betragsfelds — **drei-wertig**: ein fehlendes Vorzeichen
/// ist ein eigener Fall (`Missing`) und wird **nicht** auf positiv gedefaultet, sonst
/// ginge die terminale `SignMissing`-Bedeutung der Domäne verloren.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sign {
    Negative,
    Positive,
    Missing,
}

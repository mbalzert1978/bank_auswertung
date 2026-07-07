/// Der öffentliche, mechanische Kalender-Seam ([ADR-0008](../../../../docs/adr/0008-event-getriebene-pipeline-und-outbox.md)):
/// deutet ein rohes Datumsfeld zu seinen Bestandteilen. **Kein Domänentyp** — die Bestandteile
/// sind Primitive, sodass IO (mit `chrono`) diesen Seam ohne Domänen-Kenntnis erfüllen kann;
/// Format-/Kalenderprüfung liegt bei der Implementierung. Vorbild `ISftpPuller` der
/// C#-Referenz: ein öffentlicher, roh-typisierter Seam, den nur IO erfüllt — die
/// [`KalenderBridge`](crate::kalender_bridge::KalenderBridge) mappt ihn auf den internen
/// Domänen-Port `einlesen_domain::Kalender`, den der Handler verlangt.
pub trait KalenderSeam {
    /// Deutet den rohen Datumswert zu seinen Bestandteilen, oder `None`, wenn er kein gültiges
    /// Datum ist.
    fn deute(&self, roh: &str) -> Option<Kalendertag>;
}

/// Die Bestandteile eines gedeuteten Datums — **adapter-eigen** (kein Re-Export des
/// gleichnamigen Domänentyps `einlesen_domain::Kalendertag`): plain Felder, weil dieser Typ
/// die Adapter-Grenze überquert (Seam-Rückgabewert **und** Feldtyp des `Zeile`-DTOs) und keine
/// Domänen-Interna dorthin durchreichen soll.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Kalendertag {
    pub jahr: u16,
    pub monat: u8,
    pub tag: u8,
}

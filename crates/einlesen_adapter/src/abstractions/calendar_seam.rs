/// Der öffentliche, mechanische Kalender-Seam ([ADR-0008](../../../../docs/adr/0008-event-getriebene-pipeline-und-outbox.md)):
/// deutet ein rohes Datumsfeld zu seinen Bestandteilen. **Kein Domänentyp** — die Bestandteile
/// sind Primitive, sodass IO (mit `chrono`) diesen Seam ohne Domänen-Kenntnis erfüllen kann;
/// Format-/Kalenderprüfung liegt bei der Implementierung. Vorbild `ISftpPuller` der
/// C#-Referenz: ein öffentlicher, roh-typisierter Seam, den nur IO erfüllt — die
/// `CalendarBridge` mappt ihn auf den internen
/// Domänen-Port `einlesen_domain::Calendar`, den der Handler verlangt.
pub trait CalendarSeam {
    /// Deutet den rohen Datumswert zu seinen Bestandteilen, oder `None`, wenn er kein gültiges
    /// Datum ist.
    fn parse(&self, raw: &str) -> Option<CalendarDay>;
}

/// Die Bestandteile eines gedeuteten Datums — **adapter-eigen** (kein Re-Export des
/// gleichnamigen Domänentyps `einlesen_domain::CalendarDay`): plain Felder, weil dieser Typ
/// die Adapter-Grenze überquert (Seam-Rückgabewert **und** Feldtyp des `Line`-DTOs) und keine
/// Domänen-Interna dorthin durchreichen soll.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CalendarDay {
    pub year: u16,
    pub month: u8,
    pub day: u8,
}

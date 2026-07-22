/// Der öffentliche, mechanische Währungs-Seam ([ADR-0008](../../../../docs/adr/0008-event-getriebene-pipeline-und-outbox.md)):
/// deutet einen rohen Währungscode zu seinen Bestandteilen — dem **kanonischen ISO-4217-Code** und
/// der **Minor Unit** (Nachkommastellen). **Kein Domänentyp** — die Bestandteile sind Primitive,
/// sodass IO (mit einer battle-tested ISO-4217-Lib) diesen Seam ohne Domänen-Kenntnis erfüllen kann;
/// die Kenntnis „welche Codes gibt es, wie viele Nachkommastellen hat jeder" liegt bei der
/// Implementierung. Vorbild [`CalendarSeam`](crate::CalendarSeam): ein öffentlicher, roh-typisierter
/// Seam, den nur IO erfüllt — die `CurrencyBridge` mappt ihn auf den internen Domänen-Port
/// `einlesen_domain::Currencies`, den der Handler verlangt.
pub trait CurrencySeam {
    /// Deutet den rohen Währungscode zu seinen Bestandteilen, oder `None`, wenn es keine der Lib
    /// bekannte Währung ist.
    fn resolve(&self, code: &str) -> Option<ResolvedCurrency>;
}

/// Die Bestandteile einer gedeuteten Währung — **adapter-eigen** (kein Re-Export des Domänen-VOs
/// `einlesen_domain::Currency`): plain Felder, weil dieser Typ die Adapter-Grenze überquert
/// (Seam-Rückgabewert) und keine Domänen-Interna dorthin durchreichen soll.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedCurrency {
    /// Der kanonische ISO-4217-Code (z. B. `EUR`), wie ihn die Lib führt.
    pub code: String,
    /// Die Nachkommastellen der Minor Unit dieser Währung (EUR 2, JPY 0, BHD 3).
    pub minor_unit: u32,
}

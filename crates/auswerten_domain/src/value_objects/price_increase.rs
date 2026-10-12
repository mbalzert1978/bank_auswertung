use crate::value_objects::{Amount, DayStamp};

/// Eine **Preissteigerung** bei einem sonst gleichbleibenden wiederkehrenden Abfluss: ab einem
/// **Stichtag** wird ein höherer Betrag abgebucht als zuvor
/// ([Issue 0005](../../../../docs/issues/0005-wiederkehr-erkennung.md)).
///
/// Sie trägt **beide** Beträge, nicht bloß die Differenz: „von 12,99 auf 14,99" ist die Aussage, an
/// der ein Mensch entscheidet, ob ihm der Dienst das noch wert ist — eine nackte Differenz ließe die
/// Frage offen, worauf sie sich bezieht.
///
/// **Nur aufwärts, per Konstruktion:** der einzige Konstruktor gibt `None`, wenn `after` nicht
/// größer als `before` ist. Eine „Preissteigerung", die in Wahrheit eine Senkung ist, lässt sich
/// damit nicht bauen — nicht geprüft, sondern unkonstruierbar (ADR-0005).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PriceIncrease {
    since: DayStamp,
    before: Amount,
    after: Amount,
}

impl PriceIncrease {
    /// Die Steigerung ab `since` von `before` auf `after` — oder `None`, wenn es keine ist.
    #[must_use]
    pub(crate) fn of(since: DayStamp, before: Amount, after: Amount) -> Option<Self> {
        (after > before).then_some(Self {
            since,
            before,
            after,
        })
    }

    /// Der **Stichtag**: die erste Abbuchung zum neuen Betrag.
    #[must_use]
    pub fn since(&self) -> DayStamp {
        self.since
    }

    /// Der typische Betrag **vor** dem Stichtag.
    #[must_use]
    pub fn before(&self) -> Amount {
        self.before
    }

    /// Der typische Betrag **ab** dem Stichtag.
    #[must_use]
    pub fn after(&self) -> Amount {
        self.after
    }
}

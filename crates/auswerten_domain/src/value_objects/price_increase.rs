use crate::value_objects::{Amount, DayStamp};

/// Eine **Preissteigerung** bei einem sonst gleichbleibenden wiederkehrenden Abfluss: ab einem
/// **Stichtag** wird ein höherer Betrag abgebucht als zuvor
/// ([Issue 0005](../../../../docs/issues/0005-wiederkehr-erkennung.md)).
///
/// Sie trägt **beide** Beträge, nicht bloß die Differenz: „von 12,99 auf 14,99" ist die Aussage, an
/// der ein Mensch entscheidet, ob ihm der Dienst das noch wert ist — eine nackte Differenz ließe die
/// Frage offen, worauf sie sich bezieht.
///
/// **Vorzeichenlos und nur aufwärts:** `after` ist per Konstruktion größer als `before` — die
/// Erkennung bildet eine Preissteigerung nur aus einer Stufe nach oben. Eine Senkung bricht die
/// Reihe ab (siehe `Recurrence`), statt hier als negative Steigerung aufzutauchen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PriceIncrease {
    since: DayStamp,
    before: Amount,
    after: Amount,
}

impl PriceIncrease {
    #[must_use]
    pub(crate) const fn of(since: DayStamp, before: Amount, after: Amount) -> Self {
        Self {
            since,
            before,
            after,
        }
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

use crate::value_objects::{AccountToken, Amount, DayStamp};

/// Eine **Abbuchung** einer Buchungsart — das, was die Wiederkehr-Erkennung von einer Buchung
/// braucht. Die Feldreihenfolge ist die Sortier-Ordnung: erst der Tag (die Zeitachse), dann die Zahl,
/// dann das Konto — total, damit die Reihe eingabe-unabhängig geordnet ist.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Charge {
    day: DayStamp,
    amount: Amount,
    account: AccountToken,
}

impl Charge {
    pub(crate) fn new(day: DayStamp, amount: Amount, account: AccountToken) -> Self {
        Self {
            day,
            amount,
            account,
        }
    }

    pub(crate) fn day(&self) -> DayStamp {
        self.day
    }

    pub(crate) fn amount(&self) -> Amount {
        self.amount
    }
}

/// Die Abbuchungen **einer** Buchungsart als **chronologische, nicht-leere Reihe** — beides per
/// Konstruktion, nicht per Prüfung (ADR-0005).
///
/// **Warum ein eigener Typ.** Takt und Preisstufen lesen die Abstände **zwischen** aufeinander
/// folgenden Abbuchungen. Auf einer unsortierten Liste ergäben sich dabei negative Abstände, die
/// sättigend still zu null würden — eine falsche Aussage ohne Fehlschlag. Hier gibt es keinen Weg,
/// die Reihe ungeordnet zu bauen; und weil sie nie leer ist, hat sie immer eine **jüngste** Abbuchung.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChargeSeries {
    /// Die älteste Abbuchung — getrennt geführt, damit die Reihe nicht leer sein kann.
    first: Charge,
    /// Alle übrigen, chronologisch nach `first`.
    rest: Vec<Charge>,
}

impl ChargeSeries {
    /// Ordnet die Abbuchungen chronologisch; `None`, wenn es keine gibt.
    pub(crate) fn of(mut charges: Vec<Charge>) -> Option<Self> {
        charges.sort();
        let mut charges = charges.into_iter();
        Some(Self {
            first: charges.next()?,
            rest: charges.collect(),
        })
    }

    /// Die Abbuchungen in chronologischer Reihenfolge.
    pub(crate) fn iter(&self) -> impl Iterator<Item = &Charge> {
        std::iter::once(&self.first).chain(&self.rest)
    }

    /// Die Abbuchungstage in chronologischer Reihenfolge.
    pub(crate) fn days(&self) -> impl Iterator<Item = DayStamp> + '_ {
        self.iter().map(Charge::day)
    }

    pub(crate) fn len(&self) -> usize {
        self.rest.len().saturating_add(1)
    }

    /// Die älteste Abbuchung — immer vorhanden.
    pub(crate) fn first(&self) -> &Charge {
        &self.first
    }

    /// Das Konto der **jüngsten** Abbuchung — immer vorhanden, weil die Reihe nie leer ist.
    pub(crate) fn latest_account(&self) -> &AccountToken {
        &self.rest.last().unwrap_or(&self.first).account
    }
}

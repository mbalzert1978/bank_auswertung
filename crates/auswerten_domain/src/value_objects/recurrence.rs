use crate::value_objects::{AccountToken, Amount, DayStamp, Fingerprint, PriceIncrease, Rhythm};

/// Wie weit ein Betrag vom **Bezugsbetrag** seiner Preisstufe abweichen darf und noch als „ähnlich"
/// gilt — in Prozent des Bezugsbetrags, beide Richtungen.
///
/// **Fünf Prozent, bewusst knapp.** Abos und Daueraufträge buchen auf den Cent genau ab; die Toleranz
/// fängt nur Rundungs- und Kursreste ab. Großzügiger darf sie nicht sein, denn **was über ihr liegt,
/// ist eine Preissteigerung**: die üblichen Erhöhungen (12,99 → 13,99 sind +7,7 %) müssen oberhalb
/// bleiben, sonst verschluckte die Ähnlichkeit genau das, was ausgewiesen werden soll. Eine Erhöhung
/// um weniger als fünf Prozent bleibt darum unerkannt — sie geht als Schwankung in der Stufe auf.
const AMOUNT_TOLERANCE_PERCENT: u64 = 5;

/// Ein **wiederkehrender Abfluss** — eine Buchungs*art*, die im festen Takt zu einem ähnlichen
/// Betrag abgebucht wird ([Issue 0005](../../../../docs/issues/0005-wiederkehr-erkennung.md)): ein Abo,
/// ein Dauerauftrag, eine Versicherung.
///
/// **Ein Wert, keine Entität, und nie abgelegt:** er entsteht bei jedem Lauf der
/// fenster-übergreifenden Reichweite neu aus den unveränderlichen Aggregaten (ADR-0013, Entscheidung
/// E). Darum ist er von Natur aus replay-deterministisch und braucht keine Dedupe-Maschinerie.
///
/// Der **Jahresbetrag** ist die Bezugsgröße, auf der ein späterer Spar-Kandidat steht
/// ([Issue 0007](../../../../docs/issues/0007-aderlass-pareto-ranking-und-spar-kandidaten.md)): ein
/// Monatsabo zu 13 € und ein Jahresabo zu 150 € sind erst über ihn vergleichbar. Er rechnet mit dem
/// **aktuellen** Betrag — nach einer Preissteigerung zählt, was künftig abgeht, nicht, was früher abging.
///
/// **Datenschutz:** er trägt den [`Fingerprint`] (öffentliche Gläubiger-ID, seed-basiertes Pseudonym
/// oder Händler-Token aus gescrubbtem Freitext), Beträge, einen Tag-Stamp und ein `KONTO_…`-Token —
/// nie einen Rohwert, nie den Seed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recurrence {
    fingerprint: Fingerprint,
    rhythm: Rhythm,
    typical_amount: Amount,
    price_increase: Option<PriceIncrease>,
    charged_account: AccountToken,
}

/// Eine **Abbuchung** einer Buchungsart — das, was die Erkennung von einer Buchung braucht. Die
/// Feldreihenfolge ist die Sortier-Ordnung: erst der Tag (die Zeitachse), dann die Zahl, dann das
/// Konto — total, damit die Reihe eingabe-unabhängig geordnet ist.
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
}

/// Eine **Preisstufe**: aufeinanderfolgende Abbuchungen, die ihrem ersten Betrag ähnlich sind.
struct PriceLevel {
    since: DayStamp,
    reference: Amount,
    amounts: Vec<Amount>,
}

impl PriceLevel {
    fn starting(charge: &Charge) -> Self {
        Self {
            since: charge.day,
            reference: charge.amount,
            amounts: vec![charge.amount],
        }
    }

    /// Der **typische** Betrag der Stufe: ihr unterer Median. Kein Mittelwert — der bräuchte eine
    /// Rundung, und ein einzelner Ausreißer zöge ihn; der Median ist immer ein wirklich abgebuchter
    /// Betrag.
    fn typical(&self) -> Amount {
        let mut sorted = self.amounts.clone();
        sorted.sort_unstable();
        sorted
            .get(sorted.len().saturating_sub(1) / 2)
            .copied()
            .unwrap_or(self.reference)
    }
}

impl Recurrence {
    /// **Erkennt** die Wiederkehr in den Abbuchungen **einer** Buchungsart — oder `None`, wenn sie
    /// keine ist. Die reine Kernfunktion der Wiederkehr-Erkennung; die Abbuchungen kommen schon
    /// genettet (keine Umbuchung) und nur als Abfluss herein (siehe `HistoryEvaluation`).
    ///
    /// Zwei Prüfungen, beide müssen tragen:
    /// 1. **der Takt** — jeder Abstand zwischen zwei Abbuchungen fällt in dasselbe Band, und es sind
    ///    genug Abbuchungen (siehe [`Rhythm`]);
    /// 2. **der Betrag** — die Abbuchungen zerfallen chronologisch in Preisstufen, und jede neue Stufe
    ///    liegt **über** der vorigen. Innerhalb einer Stufe ist jeder Betrag dem ersten ähnlich (±5 %).
    ///    Ein Betrag **darunter** bricht die Reihe: was mal mehr, mal weniger kostet, ist kein
    ///    gleichbleibender Abfluss, sondern ein schwankender — und den als Abo auszuweisen wäre eine
    ///    Aussage, die die Daten nicht tragen.
    ///
    /// Gibt es **mehr als eine** Stufe, ist die **jüngste** Stufung die ausgewiesene Preissteigerung:
    /// sie erklärt den heutigen Betrag.
    pub(crate) fn detect(fingerprint: Fingerprint, charges: &[Charge]) -> Option<Self> {
        let mut charges = charges.to_vec();
        charges.sort();
        let rhythm = Rhythm::of(&charges.iter().map(|charge| charge.day).collect::<Vec<_>>())?;
        let levels = price_levels(&charges)?;
        let (current, previous) = match levels.as_slice() {
            [.., previous, current] => (current, Some(previous)),
            [current] => (current, None),
            [] => return None,
        };
        Some(Self {
            fingerprint,
            rhythm,
            typical_amount: current.typical(),
            price_increase: previous.map(|previous| {
                PriceIncrease::of(current.since, previous.typical(), current.typical())
            }),
            // Das Konto der **jüngsten** Abbuchung: dort wird heute abgebucht, und dort müsste eine
            // Kündigung ansetzen. Zog das Abo einmal um, steht hier das neue Konto.
            charged_account: charges.last()?.account.clone(),
        })
    }

    #[must_use]
    pub fn fingerprint(&self) -> &Fingerprint {
        &self.fingerprint
    }

    #[must_use]
    pub fn rhythm(&self) -> Rhythm {
        self.rhythm
    }

    /// Der **typische** Betrag je Abbuchung — der der aktuellen Preisstufe.
    #[must_use]
    pub fn typical_amount(&self) -> Amount {
        self.typical_amount
    }

    /// Der **Jahresbetrag**: der typische Betrag, so oft im Jahr, wie der Takt abbucht. Abgeleitet,
    /// nicht gespeichert — er ist eine Rechnung über zwei Felder, keine eigene Aussage.
    #[must_use]
    pub fn annual_amount(&self) -> Amount {
        self.typical_amount.times(self.rhythm.charges_per_year())
    }

    #[must_use]
    pub fn price_increase(&self) -> Option<&PriceIncrease> {
        self.price_increase.as_ref()
    }

    /// Das **belastete eigene Konto** als Token — das Auftragskonto der jüngsten Abbuchung.
    #[must_use]
    pub fn charged_account(&self) -> &AccountToken {
        &self.charged_account
    }
}

/// Zerlegt die chronologischen Abbuchungen in **Preisstufen** — oder `None`, sobald ein Betrag unter
/// die laufende Stufe fällt (die Reihe ist dann kein gleichbleibender Abfluss).
fn price_levels(charges: &[Charge]) -> Option<Vec<PriceLevel>> {
    let (first, rest) = charges.split_first()?;
    rest.iter()
        .try_fold(vec![PriceLevel::starting(first)], |mut levels, charge| {
            let current = levels.last_mut()?;
            match (
                is_similar(charge.amount, current.reference),
                charge.amount > current.reference,
            ) {
                (true, _) => current.amounts.push(charge.amount),
                (false, true) => levels.push(PriceLevel::starting(charge)),
                (false, false) => return None,
            }
            Some(levels)
        })
}

/// Liegt `amount` innerhalb der Toleranz um `reference`? Gerechnet in `u128`, damit weder die
/// Differenz noch das Produkt mit 100 überlaufen kann.
fn is_similar(amount: Amount, reference: Amount) -> bool {
    let amount = u128::from(amount.minor_units());
    let reference = u128::from(reference.minor_units());
    amount.abs_diff(reference) * 100 <= reference * u128::from(AMOUNT_TOLERANCE_PERCENT)
}

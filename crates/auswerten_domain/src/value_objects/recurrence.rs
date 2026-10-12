use crate::value_objects::{
    AccountToken, Amount, Charge, ChargeSeries, CostDriver, DayStamp, PriceIncrease, Rhythm,
};

/// Wie weit ein Betrag vom **Bezugsbetrag** seiner Preisstufe abweichen darf und noch als „ähnlich"
/// gilt — in Prozent des Bezugsbetrags, beide Richtungen.
///
/// **Fünf Prozent, bewusst knapp.** Abos und Daueraufträge buchen auf den Cent genau ab; die Toleranz
/// fängt nur Rundungs- und Kursreste ab. Großzügiger darf sie nicht sein, denn **was über ihr liegt,
/// ist eine Preissteigerung**: die üblichen Erhöhungen (12,99 → 13,99 sind +7,7 %) müssen oberhalb
/// bleiben, sonst verschluckte die Ähnlichkeit genau das, was ausgewiesen werden soll. Eine Erhöhung
/// um weniger als fünf Prozent bleibt darum unerkannt — sie geht als Schwankung in der Stufe auf.
const AMOUNT_TOLERANCE_PERCENT: u64 = 5;

/// Ein **wiederkehrender Abfluss** — ein Kostentreiber ([`CostDriver`]: eine Buchungs*art* oder die
/// Gebühren eines Kontos), der im festen Takt zu einem ähnlichen Betrag abgebucht wird
/// ([Issue 0005](../../../../docs/issues/0005-wiederkehr-erkennung.md)): ein Abo, ein Dauerauftrag,
/// eine Versicherung, eine monatliche Kontoführung.
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
/// **Datenschutz:** er trägt den Treiber (Fingerabdruck — öffentliche Gläubiger-ID, seed-basiertes
/// Pseudonym oder Händler-Token aus gescrubbtem Freitext — oder ein `KONTO_…`-Token), Beträge, einen Tag-Stamp und ein `KONTO_…`-Token —
/// nie einen Rohwert, nie den Seed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recurrence {
    driver: CostDriver,
    rhythm: Rhythm,
    typical_amount: Amount,
    price_increase: Option<PriceIncrease>,
    charged_account: AccountToken,
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
            since: charge.day(),
            reference: charge.amount(),
            amounts: vec![charge.amount()],
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
    /// **Erkennt** die Wiederkehr in den Abbuchungen **eines** Treibers — oder `None`, wenn sie
    /// keine ist. Die reine Kernfunktion der Wiederkehr-Erkennung; die Abbuchungen kommen schon
    /// genettet (keine Umbuchung) und nur als Abfluss herein (siehe `HistoryEvaluation`).
    ///
    /// Zwei Prüfungen, beide müssen tragen:
    /// 1. **der Takt** — jeder Abstand zwischen zwei Abbuchungen fällt in dasselbe Band, und es sind
    ///    genug Abbuchungen (siehe [`Rhythm`]);
    /// 2. **der Betrag** — die Abbuchungen zerfallen chronologisch in Preisstufen. Innerhalb einer
    ///    Stufe ist jeder Betrag dem ersten ähnlich (±5 %); ein Betrag darüber beginnt eine neue
    ///    Stufe, einer darunter bricht die Reihe. Zusätzlich muss der **typische** Betrag jeder Stufe
    ///    über dem der vorigen liegen — sonst gälte eine Folge wie 100 / 105 / 105 / 106 / 101 / 101
    ///    als „Steigerung", obwohl der Preis gesunken ist (der Schwellwert vergleicht den ersten
    ///    Betrag einer Stufe, ausgewiesen wird aber der typische). Was mal mehr, mal weniger kostet,
    ///    ist kein gleichbleibender Abfluss — und den als Abo auszuweisen wäre eine Aussage, die die
    ///    Daten nicht tragen.
    ///
    /// Gibt es **mehr als eine** Stufe, ist die **jüngste** Stufung die ausgewiesene Preissteigerung:
    /// sie erklärt den heutigen Betrag.
    pub(crate) fn detect(driver: CostDriver, series: &ChargeSeries) -> Option<Self> {
        let rhythm = Rhythm::of(series)?;
        let levels = price_levels(series)?;
        let (current, price_increase) = match levels.as_slice() {
            [.., previous, current] => (
                current,
                Some(PriceIncrease::of(
                    current.since,
                    previous.typical(),
                    current.typical(),
                )?),
            ),
            [current] => (current, None),
            [] => return None,
        };
        Some(Self {
            driver,
            rhythm,
            typical_amount: current.typical(),
            price_increase,
            // Das Konto der **jüngsten** Abbuchung: dort wird heute abgebucht, und dort müsste eine
            // Kündigung ansetzen. Zog das Abo einmal um, steht hier das neue Konto.
            charged_account: series.latest_account().clone(),
        })
    }

    /// Der **Kostentreiber**, der wiederkehrt — eine Buchungsart oder die Gebühren eines Kontos
    /// (eine monatliche Kontoführung ist so wiederkehrend wie ein Abo).
    #[must_use]
    pub fn driver(&self) -> &CostDriver {
        &self.driver
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

/// Zerlegt die Abbuchungs-Reihe in **Preisstufen** — oder `None`, sobald die Reihe kein
/// gleichbleibender Abfluss ist: ein Betrag fällt unter die laufende Stufe, oder der **typische**
/// Betrag einer Stufe liegt nicht über dem der vorigen.
fn price_levels(series: &ChargeSeries) -> Option<Vec<PriceLevel>> {
    let levels = series.iter().skip(1).try_fold(
        vec![PriceLevel::starting(series.first())],
        |mut levels, charge| {
            let current = levels.last_mut()?;
            match (
                is_similar(charge.amount(), current.reference),
                charge.amount() > current.reference,
            ) {
                (true, _) => current.amounts.push(charge.amount()),
                (false, true) => levels.push(PriceLevel::starting(charge)),
                (false, false) => return None,
            }
            Some(levels)
        },
    )?;
    levels
        .iter()
        .zip(levels.iter().skip(1))
        .all(|(earlier, later)| later.typical() > earlier.typical())
        .then_some(levels)
}

/// Liegt `amount` innerhalb der Toleranz um `reference`? Gerechnet in `u128`, damit weder die
/// Differenz noch das Produkt mit 100 überlaufen kann.
fn is_similar(amount: Amount, reference: Amount) -> bool {
    let amount = u128::from(amount.minor_units());
    let reference = u128::from(reference.minor_units());
    amount.abs_diff(reference) * 100 <= reference * u128::from(AMOUNT_TOLERANCE_PERCENT)
}

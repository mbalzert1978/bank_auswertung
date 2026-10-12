use std::cmp::Reverse;
use std::num::NonZeroU64;

use crate::value_objects::{Amount, RankedDriver};

/// Die **Spar-Schwelle**: ab welchem **Jahresbetrag** ein Kostentreiber als Spar-Kandidat
/// vorgeschlagen wird ([Issue 0007](../../../../docs/issues/0007-aderlass-pareto-ranking-und-spar-kandidaten.md),
/// Entscheidung V3 des Abschlussplans).
///
/// **Ein Config-Wert, kein Fachwissen:** was „lohnt sich anzusehen" heißt, entscheidet der Nutzer,
/// nicht die Domäne — sie bekommt die Schwelle als **Wert** herein (io lädt sie aus der Config, die
/// Domäne liest keine Datei) und schlägt vor; entschieden wird außerhalb.
///
/// **Bezugsgröße ist der Jahresbetrag**, nicht der Betrag je Abbuchung und nicht die Summe im Lauf:
/// erst über ihn sind ein Monatsabo zu 13 € und ein Jahresabo zu 150 € vergleichbar, und ein Lauf über
/// drei Jahre hebt einen Treiber nicht dreimal so hoch wie einer über eines.
///
/// **Nie null, per Typ:** eine Schwelle von 0 machte **jeden** Treiber zum Kandidaten — keine Auswahl
/// mehr, sondern das Ranking noch einmal. [`NonZeroU64`] macht sie unkonstruierbar statt „geprüft"
/// (ADR-0005), dasselbe Muster wie die Seitengröße des Pagers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SavingsThreshold(NonZeroU64);

impl SavingsThreshold {
    /// Die Schwelle als Jahresbetrag in Minor Units der Kontowährung.
    #[must_use]
    pub fn annual(minor_units: NonZeroU64) -> Self {
        Self(minor_units)
    }

    /// Erreicht ein Jahresbetrag die Schwelle? **Einschließlich** — „ab 100 €" schließt 100 € ein.
    fn is_reached_by(self, annual_amount: Amount) -> bool {
        annual_amount.minor_units() >= self.0.get()
    }
}

/// Ein **Spar-Kandidat** — ein Kostentreiber, dessen Jahresbetrag die [`SavingsThreshold`] erreicht.
/// Ein **Vorschlag**, keine Entscheidung: das Programm benennt, was sich anzusehen lohnt; ob gespart
/// wird, entscheidet der Nutzer. Kein LLM ist beteiligt (ADR-0004) — der Kandidat ist eine Sicht auf
/// das Ranking, kein eigener Rechenschritt.
///
/// Er trägt seinen **Platz im Pareto-Ranking** und den ganzen [`RankedDriver`] — darin den
/// Jahresbetrag, den Anteil am Abfluss und, bei einem wiederkehrenden Treiber, Takt und
/// **Preissteigerung**: ein Abo, das teurer wurde, ist der naheliegendste Kandidat überhaupt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavingsCandidate {
    pareto_rank: usize,
    driver: RankedDriver,
}

impl SavingsCandidate {
    /// Der Platz des Treibers im Pareto-Ranking, **ab 1**.
    #[must_use]
    pub fn pareto_rank(&self) -> usize {
        self.pareto_rank
    }

    #[must_use]
    pub fn driver(&self) -> &RankedDriver {
        &self.driver
    }
}

/// Leitet die Spar-Kandidaten **strukturell** aus dem Ranking ab: jeder Treiber, der eine Art oder
/// eine Gebühr ist (nicht der Sammel-Treiber) und dessen Jahresbetrag die Schwelle erreicht.
///
/// Geordnet **absteigend nach Jahresbetrag** — der Größe, an der die Schwelle misst —, bei
/// Gleichstand nach dem Platz im Ranking. Die Reihenfolge weicht damit bewusst vom Ranking ab: das
/// rankt nach dem, was im Lauf abfloss, die Kandidaten nach dem, was ein Jahr kostet.
pub(crate) fn candidates(
    ranking: &[RankedDriver],
    threshold: SavingsThreshold,
) -> Vec<SavingsCandidate> {
    let mut candidates = ranking
        .iter()
        .zip(1_usize..)
        .filter(|(driver, _)| driver.driver().is_specific())
        .filter(|(driver, _)| threshold.is_reached_by(driver.annual_amount()))
        .map(|(driver, pareto_rank)| SavingsCandidate {
            pareto_rank,
            driver: driver.clone(),
        })
        .collect::<Vec<_>>();
    candidates.sort_by_key(|candidate| {
        (
            Reverse(candidate.driver.annual_amount()),
            candidate.pareto_rank,
        )
    });
    candidates
}

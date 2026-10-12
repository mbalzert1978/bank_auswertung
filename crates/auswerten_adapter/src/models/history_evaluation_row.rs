use auswerten_domain::{HistoryEvaluation, PriceIncrease, Recurrence, Rhythm};

use crate::models::{AccountCashflowRow, DriverRankRow, OccurrenceRow, SavingsCandidateRow};

/// Das **Ergebnis der fenster-übergreifenden Reichweite** als Ausgangs-DTO — die `Response` des
/// [`HistoryEvaluationHandler`](crate::HistoryEvaluationHandler) (ADR-0013, Entscheidung E,
/// Reichweite 2).
///
/// **Zurückgegeben, nie abgelegt:** es gibt kein Vorkommens-/Fold-Log. Wer die Wiederkehr braucht —
/// das Pareto-Ranking und die Spar-Kandidaten (Issue 0007), der HTML-Bericht (Issue 0012), der
/// Chat-Export (Issue 0011) —, ruft diese Reichweite auf und liest dieses DTO.
///
/// Die Sichten sind verschieden weit gefasst: die [`occurrences`](Self::occurrences) sind die
/// **ungefilterte** Zeitachse aller Buchungsarten; alles andere ist **genettet** (keine Umbuchung
/// zwischen eigenen Konten) — die [`recurrences`](Self::recurrences) als wiederkehrende Abflüsse,
/// der Aderlass je Konto und Monat ([`account_cashflows`](Self::account_cashflows)), das
/// Pareto-Ranking der Kostentreiber ([`ranking`](Self::ranking)) und die Spar-Kandidaten daraus
/// ([`savings_candidates`](Self::savings_candidates), [Issue
/// 0007](../../../../docs/issues/0007-aderlass-pareto-ranking-und-spar-kandidaten.md)).
///
/// **Für genau einen Lauf:** die Reichweite ist über den Lauf gescopt (ADR-0016) — wer den Bericht
/// eines Laufs schreibt, nimmt die Zeile, die dessen `HistorieVollständig` ergab, und rechnet nichts
/// nach.
///
/// **Egress-Kandidat** (Bericht, Export): es trägt darum die compilerfeste „kein `Serialize`"-Sperre
/// (siehe die Crate-Wurzel), dieselbe Linie wie die Anfrage ans Orakel.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HistoryEvaluationRow {
    /// Die Vorkommen der ganzen Zeitachse, chronologisch.
    pub occurrences: Vec<OccurrenceRow>,
    /// Die wiederkehrenden Abflüsse, je Buchungsart einer, nach Fingerabdruck geordnet.
    pub recurrences: Vec<RecurrenceRow>,
    /// Der genettete Cashflow je eigenem Konto und Monat — nach Konto, darin chronologisch.
    pub account_cashflows: Vec<AccountCashflowRow>,
    /// Das Pareto-Ranking der Kostentreiber, der größte Abfluss zuerst.
    pub ranking: Vec<DriverRankRow>,
    /// Die Spar-Kandidaten, der höchste Jahresbetrag zuerst.
    pub savings_candidates: Vec<SavingsCandidateRow>,
}

/// Ein **wiederkehrender Abfluss** als Ausgangs-DTO
/// ([Issue 0005](../../../../docs/issues/0005-wiederkehr-erkennung.md)): ein Abo, ein Dauerauftrag,
/// eine Versicherung — je Buchungsart eine Zeile.
///
/// **Datenschutz:** sie trägt nur den Fingerabdruck (öffentliche Gläubiger-ID, seed-basiertes
/// Pseudonym oder Händler-Token aus gescrubbtem Freitext), Beträge, einen Tag-Stamp und ein
/// `KONTO_…`-Token — nie eine IBAN, nie einen Klartextnamen, nie den Seed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecurrenceRow {
    /// Der Fingerabdruck der Buchungs*art* — bei einem Zahlungsintermediär samt Händler-Token, sodass
    /// zwei Händler hinter demselben Durchleiter zwei Zeilen ergeben.
    pub fingerprint: String,
    pub rhythm: RecurrenceRhythm,
    /// Der typische Betrag **je Abbuchung**, zum aktuellen Preis (Minor Units).
    pub typical_amount_minor_units: u64,
    /// Der **Jahresbetrag** zum aktuellen Preis (Minor Units) — die Bezugsgröße für spätere
    /// Spar-Kandidaten: erst über ihn sind Monats- und Jahresabo vergleichbar.
    pub annual_amount_minor_units: u64,
    /// Die jüngste Preissteigerung, falls der Betrag ab einem Stichtag gestiegen ist.
    pub price_increase: Option<PriceIncreaseRow>,
    /// Das **belastete eigene Konto** als `KONTO_…`-Token — das Konto der jüngsten Abbuchung.
    pub charged_account: String,
}

/// Der Takt als **adapter-eigener DTO-Typ** — bewusst re-deklariert statt aus der Domäne
/// re-exportiert, wie [`BookingDirection`](crate::BookingDirection): kein Domänen-Internum
/// überquert die Grenze.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecurrenceRhythm {
    Monthly,
    Yearly,
}

/// Eine **Preissteigerung** als Ausgangs-DTO: ab dem Stichtag wird mehr abgebucht als zuvor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PriceIncreaseRow {
    /// Der Stichtag — die erste Abbuchung zum neuen Betrag — als sortierbarer Tag-Stamp.
    pub since_day: i64,
    /// Der typische Betrag **vor** dem Stichtag (Minor Units).
    pub before_minor_units: u64,
    /// Der typische Betrag **ab** dem Stichtag (Minor Units).
    pub after_minor_units: u64,
}

impl From<&HistoryEvaluation> for HistoryEvaluationRow {
    /// Setzt das Domänen-Ergebnis auf die rohe Zeile um (ACL, „innen → außen").
    fn from(evaluation: &HistoryEvaluation) -> Self {
        Self {
            occurrences: evaluation
                .occurrences()
                .iter()
                .map(OccurrenceRow::from)
                .collect(),
            recurrences: evaluation
                .recurrences()
                .iter()
                .map(RecurrenceRow::from)
                .collect(),
            account_cashflows: evaluation
                .account_cashflows()
                .iter()
                .map(AccountCashflowRow::from)
                .collect(),
            ranking: evaluation
                .ranking()
                .iter()
                .map(DriverRankRow::from)
                .collect(),
            savings_candidates: evaluation
                .savings_candidates()
                .iter()
                .map(SavingsCandidateRow::from)
                .collect(),
        }
    }
}

impl From<&Recurrence> for RecurrenceRow {
    fn from(recurrence: &Recurrence) -> Self {
        Self {
            fingerprint: recurrence.fingerprint().as_str().to_string(),
            rhythm: rhythm_row(recurrence.rhythm()),
            typical_amount_minor_units: recurrence.typical_amount().minor_units(),
            annual_amount_minor_units: recurrence.annual_amount().minor_units(),
            price_increase: recurrence.price_increase().map(PriceIncreaseRow::from),
            charged_account: recurrence.charged_account().as_str().to_string(),
        }
    }
}

impl From<&PriceIncrease> for PriceIncreaseRow {
    fn from(increase: &PriceIncrease) -> Self {
        Self {
            since_day: increase.since().value(),
            before_minor_units: increase.before().minor_units(),
            after_minor_units: increase.after().minor_units(),
        }
    }
}

/// Der Takt „innen → außen" — die eine Abbildung, die Wiederkehr und Ranking teilen.
pub(crate) fn rhythm_row(rhythm: Rhythm) -> RecurrenceRhythm {
    match rhythm {
        Rhythm::Monthly => RecurrenceRhythm::Monthly,
        Rhythm::Yearly => RecurrenceRhythm::Yearly,
    }
}

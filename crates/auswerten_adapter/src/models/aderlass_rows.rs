use auswerten_domain::{
    AccountCashflow, Amount, CostDriver, RankedDriver, SavingsCandidate, SavingsThreshold,
};

use crate::models::account_role_vocabulary::role_label;
use crate::models::history_evaluation_row::rhythm_row;
use crate::models::{PriceIncreaseRow, RecurrenceRhythm};

/// Die **Spar-Schwelle** als Eingangs-DTO: ab welchem **Jahresbetrag** (Minor Units der
/// Kontowährung) ein Kostentreiber als Spar-Kandidat vorgeschlagen wird
/// ([Issue 0007](../../../../docs/issues/0007-aderlass-pareto-ranking-und-spar-kandidaten.md)).
///
/// Wie die [`OwnAccountsConfig`](crate::OwnAccountsConfig) ein **Wert**, den io beim Bau hereinreicht
/// — der Slice liest keine Datei. Ihre Deutung ist infallibel: jede Zahl ist eine Schwelle; dass sie
/// positiv ist, hat der Loader am Rand verlangt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SavingsThresholdConfig {
    pub annual_minor_units: u64,
}

impl SavingsThresholdConfig {
    /// Deutet das DTO in die Domänen-Schwelle (ACL, „außen → innen").
    pub(crate) fn to_threshold(self) -> SavingsThreshold {
        SavingsThreshold::annual(Amount::from_minor_units(self.annual_minor_units))
    }
}

/// Der **Aderlass eines eigenen Kontos in einem Monat** als Ausgangs-DTO — genettet: ohne
/// Umbuchungen zwischen den eigenen Konten, mit den Gebühren.
///
/// **Datenschutz:** das Konto steht als `KONTO_…`-Token da, nie als IBAN; die Rolle ist das
/// Vokabular der Config (`giro` | `haushalt` | `spar` | `vorlauf`) und fehlt, wenn die Config das
/// Konto nicht führt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountCashflowRow {
    /// Das **belastete eigene Konto** als Token.
    pub account: String,
    /// Seine Rolle laut Eigene-Konten-Config — `None`, wenn die Config es nicht führt.
    pub role: Option<String>,
    pub year: i64,
    /// Der Monat im Jahr, **1..=12** — von der Domäne per Konstruktion garantiert.
    pub month: i64,
    pub incoming_minor_units: u64,
    pub outgoing_minor_units: u64,
}

/// Ein **Kostentreiber** als adapter-eigener DTO-Typ — bewusst re-deklariert statt aus der Domäne
/// re-exportiert (dieselbe Linie wie bei [`CounterAccountRow`](crate::CounterAccountRow)), und
/// weiterhin ein **Summentyp**: eine Gebühr trägt ein Konto, eine Art einen Fingerabdruck, der
/// Sammel-Treiber nichts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CostDriverRow {
    /// Eine **Buchungsart**, geschlüsselt über ihren Fingerabdruck (Gläubiger-ID, Pseudonym oder
    /// Händler-Token hinter einem Durchleiter).
    Kind { fingerprint: String },
    /// Die **Gebühren eines Kontos** — was die Bank auf diesem `KONTO_…` selbst einzog.
    Fee { account: String },
    /// Alle Abflüsse **ohne Gegenpartei**, die keine Gebühr sind — gesammelt, damit die Anteile des
    /// Rankings das Ganze ergeben. Nie ein Spar-Kandidat.
    Unattributed,
}

/// Ein **Platz im Pareto-Ranking** als Ausgangs-DTO.
///
/// Anteile in **Basispunkten** (10 000 = 100 %), abgerundet; der kumulierte Anteil des letzten
/// Platzes ist genau 10 000. Der **Jahresbetrag** kommt aus der Wiederkehr, wenn `rhythm` gesetzt
/// ist, sonst ist er der Durchschnitt je Jahr über den beobachteten Zeitraum (bei weniger als einem
/// Jahr die Summe — nie hochgerechnet).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriverRankRow {
    pub driver: CostDriverRow,
    /// Was über diesen Treiber im ganzen Lauf abfloss (Minor Units), genettet.
    pub outflow_minor_units: u64,
    /// Wie viele Abflüsse den Treiber bilden.
    pub bookings: usize,
    pub share_basis_points: u32,
    pub cumulative_share_basis_points: u32,
    pub annual_amount_minor_units: u64,
    /// Der Takt, falls der Treiber ein wiederkehrender Abfluss ist.
    pub rhythm: Option<RecurrenceRhythm>,
    /// Die jüngste Preissteigerung eines wiederkehrenden Treibers.
    pub price_increase: Option<PriceIncreaseRow>,
}

/// Ein **Spar-Kandidat** als Ausgangs-DTO: ein Treiber, dessen Jahresbetrag die Schwelle erreicht —
/// ein Vorschlag, keine Entscheidung. Er nennt seinen Platz im Ranking (`pareto_rank`, ab 1), damit
/// ein Bericht ihn dort wiederfindet, und trägt Takt und **Preissteigerung** als Merkmale.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavingsCandidateRow {
    pub pareto_rank: usize,
    pub driver: CostDriverRow,
    pub annual_amount_minor_units: u64,
    pub share_basis_points: u32,
    pub rhythm: Option<RecurrenceRhythm>,
    pub price_increase: Option<PriceIncreaseRow>,
}

impl From<&AccountCashflow> for AccountCashflowRow {
    fn from(cashflow: &AccountCashflow) -> Self {
        Self {
            account: cashflow.account().as_str().to_string(),
            role: cashflow.role().map(|role| role_label(role).to_string()),
            year: cashflow.month().year(),
            month: cashflow.month().month_of_year(),
            incoming_minor_units: cashflow.incoming().minor_units(),
            outgoing_minor_units: cashflow.outgoing().minor_units(),
        }
    }
}

impl From<&CostDriver> for CostDriverRow {
    fn from(driver: &CostDriver) -> Self {
        match driver {
            CostDriver::Kind(fingerprint) => Self::Kind {
                fingerprint: fingerprint.as_str().to_string(),
            },
            CostDriver::Fee(account) => Self::Fee {
                account: account.as_str().to_string(),
            },
            CostDriver::Unattributed => Self::Unattributed,
        }
    }
}

impl From<&RankedDriver> for DriverRankRow {
    fn from(ranked: &RankedDriver) -> Self {
        Self {
            driver: CostDriverRow::from(ranked.driver()),
            outflow_minor_units: ranked.outflow().minor_units(),
            bookings: ranked.bookings(),
            share_basis_points: ranked.share().basis_points(),
            cumulative_share_basis_points: ranked.cumulative_share().basis_points(),
            annual_amount_minor_units: ranked.annual_amount().minor_units(),
            rhythm: ranked
                .recurrence()
                .map(|recurrence| rhythm_row(recurrence.rhythm())),
            price_increase: ranked
                .recurrence()
                .and_then(|recurrence| recurrence.price_increase())
                .map(PriceIncreaseRow::from),
        }
    }
}

impl From<&SavingsCandidate> for SavingsCandidateRow {
    fn from(candidate: &SavingsCandidate) -> Self {
        let ranked = DriverRankRow::from(candidate.driver());
        Self {
            pareto_rank: candidate.pareto_rank(),
            driver: ranked.driver,
            annual_amount_minor_units: ranked.annual_amount_minor_units,
            share_basis_points: ranked.share_basis_points,
            rhythm: ranked.rhythm,
            price_increase: ranked.price_increase,
        }
    }
}

//! Das **Bericht-DTO** und seine Zusammensetzung aus den Analyse-Ergebnissen eines Laufs.
//!
//! [`Bericht::compose`] ist eine **reine Funktion**: sie bekommt die Analyse-DTOs als Wert
//! ([`ReportInputs`]) und liefert nur noch Anzeige-Werte — Beschriftungen, Euro-Beträge, Monate,
//! Prozente. Sie rechnet **nichts** nach (kein Saldo, keine Summe, kein Ranking): **jede** Zahl im
//! Bericht stammt aus dem einen Ergebnis der fenster-übergreifenden Reichweite. Hier wird nur
//! **beschriftet, geordnet und formatiert**.
//!
//! **Beschriftung ohne Leak** (Privacy-Invarianten, die Report-Ausgabe ist ein Egress):
//!
//! - **Konten** erscheinen über ihre **Rolle** (`Giro`, `Haushalt`, …), wie die Domäne sie am
//!   Cashflow je Konto führt — nie über ihr `KONTO_…`-Token. Ein Konto ohne Rolle heißt
//!   `Konto ohne Rolle`; tragen mehrere Konten dieselbe Beschriftung, werden sie durchnummeriert.
//! - **Firmen** erscheinen über ihren **gescrubbten Firmennamen**, wie `obfuskieren` ihn abgelegt hat
//!   (er ist ein bewusster Analyse-Anker) — aber nur, wenn er **keinen** Namen der lokalen Namensliste
//!   nennt (dieselbe Erkennung wie beim Scrubbing) und keinen Scrub-Platzhalter trägt; sonst und ohne Namen über die öffentliche
//!   **Gläubiger-ID**. Ein Händler-Token hinter einem Zahlungsdienst steht daneben — so stehen die
//!   Händler getrennt. Gelesen wird all das aus dem **Anker** der Art (`KindAnchorRow`), den die Domäne
//!   ausgibt — der Fingerabdruck-Schlüssel wird nie zerlegt.
//! - **Personen** erscheinen nur über das Ende ihres seed-basierten **Pseudonyms** (`Person f30ee3`).
//! - Die **Kategorie** kommt allein aus dem Register (fehlt sie: „unbekannt").

use std::collections::{BTreeMap, BTreeSet};

use auswerten_adapter::{
    CategoryAnswerRow, CostDriverRow, HistoryEvaluationRow, KindAnchorRow, PriceIncreaseRow,
    RecurrenceRhythm,
};
use obfuskierung_adapter::{is_scrubbed, names_known_person};
use rusty_money::iso;

use crate::common::calendar;

/// Die Kategorie eines Treibers, für den das Register keinen Eintrag hat — oder der gar keinen
/// Fingerabdruck trägt (Gebühren, Abflüsse ohne Gegenpartei).
pub(crate) const UNKNOWN_CATEGORY: &str = "unbekannt";

/// Die Beschriftung eines eigenen Kontos, das die Eigene-Konten-Config nicht führt.
const ACCOUNT_WITHOUT_ROLE: &str = "Konto ohne Rolle";

/// Wie viele Zeichen des Pseudonyms eine Person im Bericht kennzeichnen — genug, um die Personen
/// eines privaten Bestands auseinanderzuhalten, kurz genug zum Lesen.
const PSEUDONYM_SHOWN: usize = 6;

/// Der Platzhalter einer leeren Zelle (kein Takt, keine Preissteigerung, keine Währung).
const NONE: &str = "—";

const MONTHS: [&str; 12] = [
    "Januar",
    "Februar",
    "März",
    "April",
    "Mai",
    "Juni",
    "Juli",
    "August",
    "September",
    "Oktober",
    "November",
    "Dezember",
];

/// Was der Bericht eines Laufs zum Zusammensetzen braucht — **reine Werte**, keine Verbindung, kein
/// Port. Ein Test baut sie aus Fixture-DTOs, ganz ohne SQLite und ohne Kategorisierer.
#[derive(Debug, Clone, Default)]
pub(crate) struct ReportInputs {
    /// Wie viele Buchungen der Analyse fehlen (`RunReport::incomplete`, ADR-0017).
    pub(crate) incomplete: usize,
    /// Das Ergebnis der fenster-übergreifenden Reichweite **dieses** Laufs — die Quelle jeder Zahl.
    pub(crate) history: HistoryEvaluationRow,
    /// Was das Kategorie-Register zu den Fingerabdrücken der Treiber kennt.
    pub(crate) categories: Vec<CategoryAnswerRow>,
    /// Gläubiger-ID → gescrubbter Firmenname, aus den Buchungen der Fenster dieses Laufs.
    pub(crate) company_names: BTreeMap<String, String>,
    /// Die lokale Namensliste (`personen.toml`): ein Firmenname, der einen davon nennt, erscheint nicht.
    pub(crate) persons: Vec<String>,
}

/// Der **Bericht** als Anzeige-DTO: nur fertig beschriftete und formatierte Werte, kein Token, keine
/// IBAN, kein Klarname einer Person, kein Seed. Er ist der **Egress** dieses Programms und trägt darum
/// die compilerfeste „kein `Serialize`"-Sperre (siehe `report::datenschutz`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Bericht {
    /// Wie viele Buchungen der Analyse fehlen — `0` heißt: keine.
    pub(crate) missing_bookings: usize,
    /// Die eine Währung des Laufs (ISO 4217) — `—` bei leerer Historie.
    pub(crate) currency: String,
    pub(crate) cashflows: Vec<CashflowLine>,
    pub(crate) transfers: Vec<TransferLine>,
    pub(crate) fees: Vec<FeeLine>,
    pub(crate) ranking: Vec<RankLine>,
    pub(crate) savings: Vec<SavingsLine>,
    pub(crate) recurrences: Vec<RecurrenceLine>,
}

/// Der genettete Cashflow eines eigenen Kontos in einem Monat.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CashflowLine {
    pub(crate) account: String,
    pub(crate) month: String,
    pub(crate) incoming: String,
    pub(crate) outgoing: String,
}

/// Die Umbuchungen eines Monats auf bzw. von einem eigenen Konto einer Rolle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TransferLine {
    pub(crate) month: String,
    pub(crate) counter_account: String,
    pub(crate) incoming: String,
    pub(crate) outgoing: String,
}

/// Die Bank-Gebühren eines Monats auf einem eigenen Konto.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FeeLine {
    pub(crate) month: String,
    pub(crate) account: String,
    pub(crate) charged: String,
    pub(crate) refunded: String,
}

/// Ein Platz im Pareto-Ranking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RankLine {
    pub(crate) rank: String,
    pub(crate) driver: String,
    pub(crate) category: String,
    pub(crate) outflow: String,
    pub(crate) bookings: String,
    pub(crate) share: String,
    pub(crate) cumulative_share: String,
    pub(crate) annual: String,
    pub(crate) rhythm: String,
}

/// Ein Spar-Kandidat — ein Vorschlag, entschieden wird vom Menschen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SavingsLine {
    pub(crate) rank: String,
    pub(crate) driver: String,
    pub(crate) category: String,
    pub(crate) annual: String,
    pub(crate) share: String,
    pub(crate) rhythm: String,
    pub(crate) price_increase: String,
}

/// Ein wiederkehrender Abfluss.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RecurrenceLine {
    pub(crate) driver: String,
    pub(crate) category: String,
    pub(crate) rhythm: String,
    pub(crate) typical: String,
    pub(crate) annual: String,
    pub(crate) price_increase: String,
    pub(crate) account: String,
}

impl Bericht {
    /// Setzt den Bericht aus den Analyse-Ergebnissen zusammen — beschriften und formatieren, nichts
    /// nachrechnen.
    pub(crate) fn compose(inputs: &ReportInputs) -> Self {
        let history = &inputs.history;
        let money = Money::of(history.currency.as_deref());
        let accounts = AccountLabels::new(history);
        let drivers = DriverLabels::new(inputs, &accounts);
        // Der teuerste wiederkehrende Abfluss zuerst; bei Gleichstand bleibt die Ordnung der Domäne.
        let mut recurrences = history.recurrences.iter().collect::<Vec<_>>();
        recurrences
            .sort_by_key(|recurrence| std::cmp::Reverse(recurrence.annual_amount_minor_units));

        let mut cashflows = history
            .account_cashflows
            .iter()
            .map(|bucket| {
                (
                    accounts.label(&bucket.account),
                    (bucket.year, bucket.month),
                    bucket,
                )
            })
            .collect::<Vec<_>>();
        // Nach der **Beschriftung** geordnet, darin chronologisch — die Token-Ordnung der Domäne ist
        // für einen Leser zufällig.
        cashflows.sort_by(|left, right| (&left.0, left.1).cmp(&(&right.0, right.1)));

        Self {
            missing_bookings: inputs.incomplete,
            currency: history.currency.clone().unwrap_or_else(|| NONE.to_string()),
            cashflows: cashflows
                .into_iter()
                .map(|(account, (year, month), bucket)| CashflowLine {
                    account,
                    month: month_label(year, month),
                    incoming: money.format(bucket.incoming_minor_units),
                    outgoing: money.format(bucket.outgoing_minor_units),
                })
                .collect(),
            transfers: history
                .transfers
                .iter()
                .map(|transfer| TransferLine {
                    month: month_label(transfer.year, transfer.month),
                    counter_account: role_label(&transfer.role),
                    incoming: money.format(transfer.incoming_minor_units),
                    outgoing: money.format(transfer.outgoing_minor_units),
                })
                .collect(),
            fees: history
                .fees
                .iter()
                .map(|fee| FeeLine {
                    month: month_label(fee.year, fee.month),
                    account: accounts.label(&fee.account),
                    charged: money.format(fee.outgoing_minor_units),
                    refunded: money.format(fee.incoming_minor_units),
                })
                .collect(),
            ranking: history
                .ranking
                .iter()
                .enumerate()
                .map(|(index, ranked)| RankLine {
                    rank: (index + 1).to_string(),
                    driver: drivers.label(&ranked.driver),
                    category: drivers.category(&ranked.driver),
                    outflow: money.format(ranked.outflow_minor_units),
                    bookings: ranked.bookings.to_string(),
                    share: percent(ranked.share_basis_points),
                    cumulative_share: percent(ranked.cumulative_share_basis_points),
                    annual: money.format(ranked.annual_amount_minor_units),
                    rhythm: rhythm_label(ranked.rhythm),
                })
                .collect(),
            savings: history
                .savings_candidates
                .iter()
                .map(|candidate| SavingsLine {
                    rank: candidate.pareto_rank.to_string(),
                    driver: drivers.label(&candidate.ranked.driver),
                    category: drivers.category(&candidate.ranked.driver),
                    annual: money.format(candidate.ranked.annual_amount_minor_units),
                    share: percent(candidate.ranked.share_basis_points),
                    rhythm: rhythm_label(candidate.ranked.rhythm),
                    price_increase: money.increase(candidate.ranked.price_increase.as_ref()),
                })
                .collect(),
            recurrences: recurrences
                .into_iter()
                .map(|recurrence| RecurrenceLine {
                    driver: drivers.label(&recurrence.driver),
                    category: drivers.category(&recurrence.driver),
                    rhythm: rhythm_label(Some(recurrence.rhythm)),
                    typical: money.format(recurrence.typical_amount_minor_units),
                    annual: money.format(recurrence.annual_amount_minor_units),
                    price_increase: money.increase(recurrence.price_increase.as_ref()),
                    account: accounts.label(&recurrence.charged_account),
                })
                .collect(),
        }
    }
}

/// Die Fingerabdrücke aller Treiber, die der Bericht zeigt — die Frage an das Kategorie-Register.
pub(crate) fn driver_fingerprints(history: &HistoryEvaluationRow) -> Vec<String> {
    history
        .ranking
        .iter()
        .map(|ranked| &ranked.driver)
        .chain(
            history
                .recurrences
                .iter()
                .map(|recurrence| &recurrence.driver),
        )
        .chain(
            history
                .savings_candidates
                .iter()
                .map(|candidate| &candidate.ranked.driver),
        )
        .filter_map(|driver| match driver {
            CostDriverRow::Kind { fingerprint, .. } => Some(fingerprint.clone()),
            CostDriverRow::Fee { .. } | CostDriverRow::Unattributed => None,
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

/// Token eigener Konten → lesbare Beschriftung über die Rolle.
struct AccountLabels(BTreeMap<String, String>);

impl AccountLabels {
    fn new(history: &HistoryEvaluationRow) -> Self {
        // Die Rolle führt die Domäne am Cashflow je Konto — gelesen aus der Eigene-Konten-Config,
        // aufgelöst auf Token. Jedes Konto, das Gebühren oder eine Wiederkehr trägt, hat dort auch
        // Abfluss, steht also mit seiner Rolle darin.
        let roles = history
            .account_cashflows
            .iter()
            .filter_map(|bucket| Some((bucket.account.as_str(), bucket.role.as_deref()?)))
            .collect::<BTreeMap<_, _>>();
        // Beschriftet werden nur Konten, die im Bericht **vorkommen**.
        let shown = history
            .account_cashflows
            .iter()
            .map(|bucket| bucket.account.as_str())
            .chain(history.fees.iter().map(|fee| fee.account.as_str()))
            .chain(
                history
                    .recurrences
                    .iter()
                    .map(|recurrence| recurrence.charged_account.as_str()),
            )
            .chain(
                history
                    .ranking
                    .iter()
                    .filter_map(|ranked| match &ranked.driver {
                        CostDriverRow::Fee { account } => Some(account.as_str()),
                        CostDriverRow::Kind { .. } | CostDriverRow::Unattributed => None,
                    }),
            )
            .collect::<BTreeSet<_>>();
        let groups =
            shown
                .into_iter()
                .fold(BTreeMap::<String, Vec<&str>>::new(), |mut groups, token| {
                    let base = roles
                        .get(token)
                        .map_or_else(|| ACCOUNT_WITHOUT_ROLE.to_string(), |role| role_label(role));
                    groups.entry(base).or_default().push(token);
                    groups
                });
        Self(
            groups
                .into_iter()
                .flat_map(|(base, tokens)| {
                    let numbered = tokens.len() > 1;
                    tokens
                        .into_iter()
                        .enumerate()
                        .map(move |(index, token)| {
                            let label = if numbered {
                                format!("{base} {}", index + 1)
                            } else {
                                base.clone()
                            };
                            (token.to_string(), label)
                        })
                        .collect::<Vec<_>>()
                })
                .collect(),
        )
    }

    /// Die Beschriftung eines Konto-Tokens — **nie** das Token selbst.
    fn label(&self, token: &str) -> String {
        self.0
            .get(token)
            .cloned()
            .unwrap_or_else(|| ACCOUNT_WITHOUT_ROLE.to_string())
    }
}

/// Kostentreiber → lesbare Beschriftung und Kategorie aus dem Register.
struct DriverLabels<'a> {
    accounts: &'a AccountLabels,
    /// Nur die Firmennamen, die **keinen** Namen der lokalen Namensliste nennen und in denen beim
    /// Obfuskieren nichts ersetzt wurde (ein `[PERSON]` darin heißt: hier stand ein Personen-Name).
    company_names: BTreeMap<&'a str, &'a str>,
    categories: BTreeMap<&'a str, &'a str>,
    /// Pseudonym → sein Endstück, so lang wie nötig, damit keine zwei Personen gleich heißen.
    person_tails: BTreeMap<&'a str, String>,
}

impl<'a> DriverLabels<'a> {
    fn new(inputs: &'a ReportInputs, accounts: &'a AccountLabels) -> Self {
        Self {
            accounts,
            company_names: inputs
                .company_names
                .iter()
                .filter(|(_, name)| {
                    !is_scrubbed(name) && !names_known_person(name, &inputs.persons)
                })
                .map(|(creditor_id, name)| (creditor_id.as_str(), name.as_str()))
                .collect(),
            categories: inputs
                .categories
                .iter()
                .map(|answer| (answer.fingerprint.as_str(), answer.category.as_str()))
                .collect(),
            person_tails: person_tails(&inputs.history),
        }
    }

    fn label(&self, driver: &CostDriverRow) -> String {
        match driver {
            CostDriverRow::Kind { anchor, .. } => self.kind_label(anchor),
            CostDriverRow::Fee { account } => {
                format!("Bankgebühren {}", self.accounts.label(account))
            }
            CostDriverRow::Unattributed => "Abflüsse ohne Gegenpartei".to_string(),
        }
    }

    fn category(&self, driver: &CostDriverRow) -> String {
        match driver {
            CostDriverRow::Kind { fingerprint, .. } => self
                .categories
                .get(fingerprint.as_str())
                .map_or(UNKNOWN_CATEGORY, |category| category)
                .to_string(),
            CostDriverRow::Fee { .. } | CostDriverRow::Unattributed => UNKNOWN_CATEGORY.to_string(),
        }
    }

    /// Die Beschriftung einer Buchungsart — aus ihrem **Anker**, den die Domäne neben dem
    /// Fingerabdruck ausgibt; der Schlüssel selbst wird nie zerlegt.
    fn kind_label(&self, anchor: &KindAnchorRow) -> String {
        match anchor {
            KindAnchorRow::Company {
                creditor_id,
                merchant,
            } => {
                let company = self.company_names.get(creditor_id.as_str()).map_or_else(
                    || format!("Gläubiger-ID {creditor_id}"),
                    |name| (*name).to_string(),
                );
                match merchant {
                    Some(merchant) => format!("{company} › {merchant}"),
                    None => company,
                }
            }
            // Das **Ende** des Pseudonyms: es ist der seed-basierte Hash-Teil, dessen Form am Anfang
            // (einem festen Präfix) hier niemand kennen muss.
            KindAnchorRow::Person { pseudonym } => format!(
                "Person {}",
                self.person_tails
                    .get(pseudonym.as_str())
                    .map_or_else(|| tail(pseudonym, PSEUDONYM_SHOWN), Clone::clone)
            ),
        }
    }
}

/// Die Endstücke aller Pseudonyme, die der Bericht zeigt: je mindestens [`PSEUDONYM_SHOWN`] Zeichen,
/// und so viele mehr, bis es sich von jedem anderen Endstück unterscheidet (höchstens das ganze
/// Pseudonym).
fn person_tails(history: &HistoryEvaluationRow) -> BTreeMap<&str, String> {
    let pseudonyms = history
        .ranking
        .iter()
        .map(|ranked| &ranked.driver)
        .chain(
            history
                .recurrences
                .iter()
                .map(|recurrence| &recurrence.driver),
        )
        .chain(
            history
                .savings_candidates
                .iter()
                .map(|candidate| &candidate.ranked.driver),
        )
        .filter_map(|driver| match driver {
            CostDriverRow::Kind {
                anchor: KindAnchorRow::Person { pseudonym },
                ..
            } => Some(pseudonym.as_str()),
            CostDriverRow::Kind { .. }
            | CostDriverRow::Fee { .. }
            | CostDriverRow::Unattributed => None,
        })
        .collect::<BTreeSet<_>>();
    pseudonyms
        .iter()
        .map(|&pseudonym| {
            let length = (PSEUDONYM_SHOWN..pseudonym.chars().count())
                .find(|&length| {
                    let own = tail(pseudonym, length);
                    pseudonyms
                        .iter()
                        .all(|&other| other == pseudonym || tail(other, length) != own)
                })
                .unwrap_or_else(|| pseudonym.chars().count());
            (pseudonym, tail(pseudonym, length))
        })
        .collect()
}

/// Die letzten `length` Zeichen eines Pseudonyms — der seed-basierte Hash-Teil; ein festes Präfix
/// muss der Bericht so nicht kennen.
fn tail(pseudonym: &str, length: usize) -> String {
    let skip = pseudonym.chars().count().saturating_sub(length);
    pseudonym.chars().skip(skip).collect()
}

/// Beträge in der Schreibweise der Währung: Tausenderpunkt, Dezimalkomma, Nachkommastellen nach
/// ISO 4217 (`rusty_money`, wie beim Einlesen).
struct Money {
    exponent: u32,
    unit: String,
}

impl Money {
    /// Nachkommastellen und Symbol nach ISO 4217. Ohne Währung gibt es keine einzige Buchung und
    /// darum keinen Betrag; eine unbekannte Währung zeigt ihren Code.
    fn of(currency: Option<&str>) -> Self {
        let code = currency.unwrap_or_default().to_ascii_uppercase();
        iso::find(&code).map_or(
            Self {
                exponent: 2,
                unit: code.clone(),
            },
            |found| Self {
                exponent: found.exponent,
                unit: found.symbol.to_string(),
            },
        )
    }

    fn format(&self, minor_units: u64) -> String {
        let divisor = 10_u64.checked_pow(self.exponent).unwrap_or(1);
        let whole = grouped(minor_units / divisor);
        match usize::try_from(self.exponent) {
            Ok(width) if width > 0 => {
                format!("{whole},{:0width$} {}", minor_units % divisor, self.unit)
            }
            _ => format!("{whole} {}", self.unit),
        }
    }

    fn increase(&self, increase: Option<&PriceIncreaseRow>) -> String {
        increase.map_or_else(
            || NONE.to_string(),
            |increase| {
                format!(
                    "seit {}: {} → {}",
                    calendar::german_day(increase.since_day).unwrap_or_else(|| NONE.to_string()),
                    self.format(increase.before_minor_units),
                    self.format(increase.after_minor_units)
                )
            },
        )
    }
}

/// Eine ganze Zahl mit Tausenderpunkten.
fn grouped(value: u64) -> String {
    let digits = value.to_string().chars().rev().collect::<Vec<_>>();
    digits
        .chunks(3)
        .map(|chunk| chunk.iter().rev().collect::<String>())
        .rev()
        .collect::<Vec<_>>()
        .join(".")
}

/// Ein Anteil in Basispunkten als Prozent mit zwei Nachkommastellen.
fn percent(basis_points: u32) -> String {
    format!("{},{:02} %", basis_points / 100, basis_points % 100)
}

/// „März 2026" — ein Monat außerhalb 1..=12 (von der Domäne ausgeschlossen) fällt auf die Zahl zurück.
fn month_label(year: i64, month: i64) -> String {
    month
        .checked_sub(1)
        .and_then(|index| usize::try_from(index).ok())
        .and_then(|index| MONTHS.get(index))
        .map_or_else(
            || format!("{month:02}/{year}"),
            |name| format!("{name} {year}"),
        )
}

/// Die Rolle (`giro`, …) in Anzeige-Form (`Giro`, …) — nur großgeschrieben, kein zweites Vokabular.
fn role_label(role: &str) -> String {
    let mut chars = role.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(chars).collect()
    })
}

fn rhythm_label(rhythm: Option<RecurrenceRhythm>) -> String {
    match rhythm {
        Some(RecurrenceRhythm::Monthly) => "monatlich",
        Some(RecurrenceRhythm::Yearly) => "jährlich",
        None => NONE,
    }
    .to_string()
}

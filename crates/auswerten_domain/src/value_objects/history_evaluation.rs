use std::collections::BTreeMap;

use crate::value_objects::{
    AccountCashflow, AccountToken, Charge, ChargeSeries, CostDriver, CounterAccount, Direction,
    Fingerprint, Month, MonthlyCashflow, Occurrence, RankedDriver, Recurrence, SavingsCandidate,
    SavingsThreshold, VerifiedOwnAccounts, WindowedBooking, rank_drivers, savings_candidates,
};

/// Das **Ergebnis der fenster-übergreifenden Reichweite** (ADR-0013, Entscheidung E, Reichweite 2):
/// die Vorkommen der ganzen Zeitachse des Laufs, die **wiederkehrenden Abflüsse**, die darin
/// stecken ([Issue 0005](../../../../docs/issues/0005-wiederkehr-erkennung.md)), und darauf der
/// **Aderlass**: Cashflow je Konto und Monat, das Pareto-Ranking der Kostentreiber und die
/// Spar-Kandidaten ([Issue 0007](../../../../docs/issues/0007-aderlass-pareto-ranking-und-spar-kandidaten.md)).
///
/// **Ein Wert, zurückgegeben, nie abgelegt:** keine Identität, kein Vorkommens-/Fold-Log, keine
/// zweite persistierte Sicht. Es entsteht bei jedem Lauf neu aus den unveränderlichen Aggregaten —
/// und ist darum replay-deterministisch, ohne Dedupe-Logik.
///
/// **Warum der Aderlass hier steht und nicht pro Fenster.** Ein Fenster ist eine ~400-Tage-*Seite*
/// des Pagers, kein Berichtszeitraum. Ein Ranking je Seite rankte nur einen Ausschnitt, und ein Monat
/// an der Seitengrenze zerfiele in zwei halbe Buckets; wer daraus das Bild des Laufs bauen wollte,
/// müsste Rankings zusammenführen und Anteile neu rechnen — Domänenwissen im Bericht. Hier sieht die
/// Reichweite **alle** Buchungen des Laufs auf einmal, und der Bericht liest fertige Zahlen.
///
/// **Die Sichten sehen verschieden viel, bewusst.** Die [`occurrences`](Self::occurrences) sind die
/// **ungefilterte** Zeitachse: jede Buchung mit Gegenpartei, in beiden Richtungen, auch die
/// Umbuchung. Alles andere ist **genettet** — eine Umbuchung aufs eigene Konto ist kein verlorenes
/// Geld: die [`recurrences`](Self::recurrences) und das [`ranking`](Self::ranking) sehen nur
/// Abflüsse, die nicht auf ein eigenes Konto gingen, die [`account_cashflows`](Self::account_cashflows)
/// beide Richtungen ohne Umbuchungen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryEvaluation {
    occurrences: Vec<Occurrence>,
    recurrences: Vec<Recurrence>,
    account_cashflows: Vec<AccountCashflow>,
    ranking: Vec<RankedDriver>,
    savings_candidates: Vec<SavingsCandidate>,
}

impl HistoryEvaluation {
    /// **Wertet die Historie aus** — der einzige Konstruktor.
    ///
    /// Für die Wiederkehr geht jede Buchung durch zwei Filter, und beide sind fachlich:
    /// - **nur Abflüsse** — ein regelmäßiger Zufluss (das Gehalt) ist kein Kosten-Treiber;
    /// - **genettet** ([Issue 0004](../../../../docs/issues/0004-umbuchung-netting.md)) — ein
    ///   Dauerauftrag aufs eigene Sparkonto ist kein verlorenes Geld, sondern eine Umbuchung, und
    ///   dieselbe Regel, die ihn aus dem Cashflow nimmt, nimmt ihn hier aus der Wiederkehr.
    ///
    /// Was bleibt, wird **je Fingerabdruck** gruppiert — dem Schlüssel einer Buchungs*art*, der bei
    /// einem Zahlungsintermediär schon den Händler-Token trägt. Zwei Händler hinter demselben
    /// Durchleiter landen darum in zwei Gruppen, ohne dass hier etwas dafür geschieht. Eine Buchung
    /// **ohne** Fingerabdruck (Entgeltzeile ohne Gegenpartei) gehört zu keiner Art und fällt heraus;
    /// was die Bank regelmäßig einzieht, weist die Gebühren-Sicht der pro-Fenster-Reichweite aus.
    ///
    /// Auf denselben genetteten Abflüssen steht das **Pareto-Ranking** (siehe [`CostDriver`]: je Art,
    /// je Gebühren-Konto, dazu ein Sammel-Treiber ohne Gegenpartei) und daraus die **Spar-Kandidaten**
    /// über der `threshold`. Der Jahresbetrag eines wiederkehrenden Treibers kommt aus seiner
    /// Wiederkehr; für jeden anderen ist er der Durchschnitt je Jahr über den **beobachteten
    /// Zeitraum** — vom ersten bis zum letzten Buchungstag des Laufs, über alle Konten.
    #[must_use]
    pub(crate) fn of(
        bookings: &[WindowedBooking],
        own_accounts: VerifiedOwnAccounts<'_>,
        threshold: SavingsThreshold,
    ) -> Self {
        let mut occurrences = bookings
            .iter()
            .filter_map(occurrence_of)
            .collect::<Vec<_>>();
        // **Chronologisch** — die Zeitachse ist der Sinn dieser Sicht. Die Ordnung ist **total**
        // (Tag, dann Schlüssel, dann Zahlen), die Sortierung also eingabe-unabhängig.
        occurrences.sort();
        // Die `BTreeMap` gruppiert je Art und legt die Gruppen zugleich in stabiler,
        // eingabe-unabhängiger Reihenfolge ab — dieselben Aggregate ergeben dieselbe Liste.
        let recurrences = netted_outflows(bookings, &own_accounts)
            .filter_map(|booking| {
                Some((
                    Fingerprint::of(booking.counterparty())?,
                    Charge::new(
                        booking.day(),
                        booking.amount(),
                        booking.ordering_account().clone(),
                    ),
                ))
            })
            .fold(
                BTreeMap::<Fingerprint, Vec<Charge>>::new(),
                |mut by_kind, (fingerprint, charge)| {
                    by_kind.entry(fingerprint).or_default().push(charge);
                    by_kind
                },
            )
            .into_iter()
            .filter_map(|(fingerprint, charges)| {
                Recurrence::detect(fingerprint, &ChargeSeries::of(charges)?)
            })
            .collect::<Vec<_>>();
        let ranking = rank_drivers(
            netted_outflows(bookings, &own_accounts)
                .map(|booking| (CostDriver::of(booking), booking.amount())),
            &recurrences,
            observed_days(bookings),
        );
        Self {
            occurrences,
            account_cashflows: account_cashflows(bookings, &own_accounts),
            savings_candidates: savings_candidates(&ranking, threshold),
            ranking,
            recurrences,
        }
    }

    /// Die Auswertung einer **leeren** Historie: keine Vorkommen, keine Wiederkehr.
    #[must_use]
    pub(crate) fn empty() -> Self {
        Self {
            occurrences: Vec::new(),
            recurrences: Vec::new(),
            account_cashflows: Vec::new(),
            ranking: Vec::new(),
            savings_candidates: Vec::new(),
        }
    }

    /// Die Vorkommen der ganzen Zeitachse, chronologisch — ungefiltert (siehe Typ-Doku).
    #[must_use]
    pub fn occurrences(&self) -> &[Occurrence] {
        &self.occurrences
    }

    /// Die **wiederkehrenden Abflüsse** des Laufs, je Buchungsart einer, in stabiler Reihenfolge des
    /// Fingerabdrucks. Wer am meisten kostet, sagt nicht diese Liste, sondern das
    /// [`ranking`](Self::ranking) — dort steht jede Wiederkehr an ihrem Treiber.
    #[must_use]
    pub fn recurrences(&self) -> &[Recurrence] {
        &self.recurrences
    }

    /// Der **Aderlass je eigenem Konto und Monat** über den ganzen Lauf, genettet — geordnet nach
    /// Konto, darin chronologisch.
    #[must_use]
    pub fn account_cashflows(&self) -> &[AccountCashflow] {
        &self.account_cashflows
    }

    /// Das **Pareto-Ranking** der Kostentreiber des Laufs, der größte zuerst — mit Anteil und
    /// kumuliertem Anteil am genetteten Abfluss.
    #[must_use]
    pub fn ranking(&self) -> &[RankedDriver] {
        &self.ranking
    }

    /// Die **Spar-Kandidaten**: die Treiber, deren Jahresbetrag die Schwelle erreicht, der teuerste
    /// zuerst. Vorschläge — entschieden wird außerhalb.
    #[must_use]
    pub fn savings_candidates(&self) -> &[SavingsCandidate] {
        &self.savings_candidates
    }
}

/// Verdichtet eine gelesene Buchung zu ihrem Vorkommen. Eine Buchung **ohne** Gegenpartei
/// (Entgeltzeile) trägt **keines** bei: es gibt nichts, was ihre Art unterscheidbar machte, und ein
/// Platzhalter-Schlüssel verklumpte alle Entgeltzeilen zu einer scheinbar wiederkehrenden Art.
fn occurrence_of(booking: &WindowedBooking) -> Option<Occurrence> {
    Fingerprint::of(booking.counterparty()).map(|fingerprint| {
        Occurrence::new(
            booking.day(),
            fingerprint,
            booking.direction(),
            booking.amount(),
        )
    })
}

/// Die **genetteten Abflüsse**: was abfloss und nicht auf ein eigenes Konto ging — die eine Menge,
/// auf der Wiederkehr und Ranking stehen. Sie steht einmal da, damit die beiden nicht auseinanderlaufen
/// können: ein Dauerauftrag aufs Sparkonto fehlt in beiden oder in keinem.
fn netted_outflows<'a>(
    bookings: &'a [WindowedBooking],
    own_accounts: &'a VerifiedOwnAccounts<'_>,
) -> impl Iterator<Item = &'a WindowedBooking> {
    bookings
        .iter()
        .filter(|booking| booking.direction() == Direction::Outgoing)
        .filter(|booking| booking.transfer_role(own_accounts).is_none())
}

/// Faltet die Buchungen ohne Umbuchungen zu **Monats-Buckets je belastetem Konto**. Der Schlüssel
/// steht als (Konto, Monat): die `BTreeMap` ordnet so zuerst nach Konto und darin chronologisch — die
/// Form, in der ein Bericht je Konto eine Zeitreihe zeigt.
fn account_cashflows(
    bookings: &[WindowedBooking],
    own_accounts: &VerifiedOwnAccounts<'_>,
) -> Vec<AccountCashflow> {
    bookings
        .iter()
        .filter(|booking| booking.transfer_role(own_accounts).is_none())
        .fold(
            BTreeMap::<(AccountToken, Month), MonthlyCashflow>::new(),
            |mut buckets, booking| {
                let month = Month::of(booking.day());
                let bucket = buckets
                    .entry((booking.ordering_account().clone(), month))
                    .or_insert_with(|| MonthlyCashflow::empty(month));
                *bucket = bucket.plus(booking.amount(), booking.direction());
                buckets
            },
        )
        .into_iter()
        .map(|((account, _month), flow)| {
            let role = match own_accounts.resolve(&account) {
                CounterAccount::Own(role) => Some(role),
                CounterAccount::Foreign => None,
            };
            AccountCashflow::of(account, role, flow)
        })
        .collect()
}

/// Die Länge des **beobachteten Zeitraums** in Tagen: vom ersten bis zum letzten Buchungstag des
/// Laufs, beide eingeschlossen — über **alle** Buchungen und Konten, denn gefragt ist, wie lange die
/// Daten reichen, nicht wie lange ein einzelner Treiber zahlte.
fn observed_days(bookings: &[WindowedBooking]) -> u64 {
    let days = bookings.iter().map(WindowedBooking::day);
    match (days.clone().min(), days.max()) {
        (Some(first), Some(last)) => {
            u64::try_from(last.value().saturating_sub(first.value()).saturating_add(1)).unwrap_or(0)
        }
        _ => 0,
    }
}

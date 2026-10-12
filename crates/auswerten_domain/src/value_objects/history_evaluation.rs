use std::collections::BTreeMap;

use crate::value_objects::{
    AccountCashflow, AccountToken, Currency, Direction, DriverGroups, Fee, Fingerprint,
    InternalTransfer, Month, MonthlyCashflow, Occurrence, RankedDriver, Recurrence,
    SavingsCandidate, SavingsThreshold, VerifiedOwnAccounts, WindowedBooking, fees,
    internal_transfers, savings_candidates,
};

/// Das **Ergebnis der fenster-übergreifenden Reichweite** (ADR-0013, Entscheidung E, Reichweite 2):
/// die Vorkommen der ganzen Zeitachse des Laufs, die **wiederkehrenden Abflüsse**, die darin
/// stecken ([Issue 0005](../../../../docs/issues/0005-wiederkehr-erkennung.md)), und darauf der
/// **Aderlass**: Cashflow je Konto und Monat, das Pareto-Ranking der Kostentreiber und die
/// Spar-Kandidaten ([Issue 0007](../../../../docs/issues/0007-aderlass-pareto-ranking-und-spar-kandidaten.md)),
/// daneben die **Umbuchungen** je Monat und Gegenkonto-Rolle und die **Gebühren** je Monat und
/// belastetem Konto über den ganzen Lauf — so liest der Bericht
/// ([Issue 0012](../../../../docs/issues/0012-html-bericht.md)) **jede** Zahl aus diesem einen Wert.
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
/// beide Richtungen ohne Umbuchungen. Die [`transfers`](Self::transfers) sind genau das, was das
/// Netting herausnahm, die [`fees`](Self::fees) ein Ausschnitt der Cashflows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryEvaluation {
    /// Die **eine** Währung aller Beträge des Laufs — `None` nur für die leere Historie.
    currency: Option<Currency>,
    occurrences: Vec<Occurrence>,
    recurrences: Vec<Recurrence>,
    account_cashflows: Vec<AccountCashflow>,
    transfers: Vec<InternalTransfer>,
    fees: Vec<Fee>,
    ranking: Vec<RankedDriver>,
    savings_candidates: Vec<SavingsCandidate>,
}

impl HistoryEvaluation {
    /// **Wertet die Historie aus** — der einzige Konstruktor, und eine kleine Pipeline aus benannten
    /// reinen Schritten:
    ///
    /// ```text
    /// netted_outflows ─► DriverGroups ─┬─► recurrences
    ///                                  └─► rank ─► savings_candidates
    /// account_cashflows   transfers   fees   occurrences   (daneben, über alle Buchungen)
    /// ```
    ///
    /// **Genettet** heißt: nur Abflüsse, die nicht auf ein eigenes Konto gingen ([Issue
    /// 0004](../../../../docs/issues/0004-umbuchung-netting.md)) — ein Gehalt ist kein Kosten-Treiber,
    /// ein Dauerauftrag aufs Sparkonto kein verlorenes Geld. Sie werden **einmal** nach
    /// [`CostDriver`](crate::CostDriver) gruppiert (je Art, je Gebühren-Konto, dazu der Sammel-Treiber ohne
    /// Gegenpartei); aus dieser **einen** Gruppierung kommen die Wiederkehr je bestimmtem Treiber
    /// (auch eine monatliche Kontoführung) und das Pareto-Ranking. Der Jahresbetrag eines
    /// wiederkehrenden Treibers kommt aus seiner Wiederkehr, für jeden anderen ist er der
    /// Durchschnitt je Jahr über den **beobachteten Zeitraum** — vom ersten bis zum letzten
    /// Buchungstag des Laufs, über alle Konten. Die Spar-Kandidaten sind die Treiber über der
    /// `threshold`.
    #[must_use]
    pub(crate) fn of(
        currency: &Currency,
        bookings: &[WindowedBooking],
        own_accounts: VerifiedOwnAccounts<'_>,
        threshold: SavingsThreshold,
    ) -> Self {
        let groups = DriverGroups::of(netted_outflows(bookings, &own_accounts));
        let ranking = groups.rank(observed_days(bookings));
        Self {
            currency: Some(currency.clone()),
            occurrences: occurrences(bookings),
            recurrences: groups.recurrences(),
            account_cashflows: account_cashflows(bookings, &own_accounts),
            transfers: internal_transfers(bookings, &own_accounts),
            fees: fees(bookings, &own_accounts),
            savings_candidates: savings_candidates(&ranking, threshold),
            ranking,
        }
    }

    /// Die Auswertung einer **leeren** Historie: keine Währung, keine Vorkommen, keine Wiederkehr.
    #[must_use]
    pub(crate) fn empty() -> Self {
        Self {
            currency: None,
            occurrences: Vec::new(),
            recurrences: Vec::new(),
            account_cashflows: Vec::new(),
            transfers: Vec::new(),
            fees: Vec::new(),
            ranking: Vec::new(),
            savings_candidates: Vec::new(),
        }
    }

    /// Die **eine** Währung aller Beträge — `None` nur, wenn die Historie leer war.
    #[must_use]
    pub fn currency(&self) -> Option<&Currency> {
        self.currency.as_ref()
    }

    /// Die Vorkommen der ganzen Zeitachse, chronologisch — ungefiltert (siehe Typ-Doku).
    #[must_use]
    pub fn occurrences(&self) -> &[Occurrence] {
        &self.occurrences
    }

    /// Die **wiederkehrenden Abflüsse** des Laufs, je Kostentreiber einer (Buchungsart oder Gebühren
    /// eines Kontos), in der stabilen Ordnung des Treibers. Wer am meisten kostet, sagt nicht diese Liste, sondern das
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

    /// Die **Umbuchungen** zwischen den eigenen Konten über den ganzen Lauf — je Monat und Rolle des
    /// Gegenkontos ein Bucket, chronologisch. Was aus den [`account_cashflows`](Self::account_cashflows)
    /// herausfiel, steht hier.
    #[must_use]
    pub fn transfers(&self) -> &[InternalTransfer] {
        &self.transfers
    }

    /// Die **Bank-Gebühren** über den ganzen Lauf — je Monat und belastetem Konto ein Bucket,
    /// chronologisch. Ein **Ausschnitt** der [`account_cashflows`](Self::account_cashflows), keine
    /// Ergänzung: eine Gebühr ist echter Abfluss und steht auch dort.
    #[must_use]
    pub fn fees(&self) -> &[Fee] {
        &self.fees
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

/// Die **Zeitachse**: je Buchung mit Gegenpartei ein Vorkommen, **chronologisch** — die Ordnung ist
/// total (Tag, dann Schlüssel, dann Zahlen), die Sortierung also eingabe-unabhängig.
fn occurrences(bookings: &[WindowedBooking]) -> Vec<Occurrence> {
    let mut occurrences = bookings
        .iter()
        .filter_map(occurrence_of)
        .collect::<Vec<_>>();
    occurrences.sort();
    occurrences
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
/// auf der Wiederkehr und Ranking stehen (über [`DriverGroups`], einmal durchlaufen).
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
            let role = own_accounts.resolve(&account).own_role();
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

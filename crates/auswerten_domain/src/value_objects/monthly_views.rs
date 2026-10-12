use std::collections::BTreeMap;

use crate::value_objects::{
    BookingKind, Fee, InternalTransfer, Month, MonthlyCashflow, VerifiedOwnAccounts,
    WindowedBooking,
};

/// Die **Umbuchungen** einer Menge von Buchungen — je Monat und **Rolle des Gegenkontos** ein Bucket,
/// chronologisch und darin nach Rolle. Die andere Seite des Nettings: was aus dem Cashflow herausfiel,
/// steht hier, statt verloren zu sein. Ein Abfluss aufs Sparkonto und einer aufs Haushaltskonto sind
/// nicht dieselbe Bewegung.
///
/// Dieselbe Funktion für **beide** Reichweiten: je Fenster (Reichweite 1) und über den ganzen Lauf
/// (Reichweite 2) — so kann die eine Sicht der anderen nicht davonlaufen.
pub(crate) fn internal_transfers(
    bookings: &[WindowedBooking],
    own_accounts: &VerifiedOwnAccounts<'_>,
) -> Vec<InternalTransfer> {
    bucket_by_month(
        bookings
            .iter()
            .filter_map(|booking| Some((booking.transfer_role(own_accounts)?, booking))),
    )
    .into_iter()
    .map(|((_month, role), flow)| InternalTransfer::of(role, flow))
    .collect()
}

/// Die **Bank-Gebühren** einer Menge von Buchungen — je Monat und **belastetem Konto** ein Bucket,
/// chronologisch und darin nach Konto-Token ([Issue 0006](../../../../docs/issues/0006-gebuehren-klassifikation.md)).
///
/// Gelesen aus **derselben** Menge wie der genettete Cashflow (ohne Umbuchungen) — nur so gilt „die
/// Gebühren sind ein Teil des Cashflows" von Bauart wegen und nicht bloß in der Praxis (siehe
/// [`Fee`]). Wie [`internal_transfers`] für beide Reichweiten dieselbe.
pub(crate) fn fees(
    bookings: &[WindowedBooking],
    own_accounts: &VerifiedOwnAccounts<'_>,
) -> Vec<Fee> {
    bucket_by_month(
        bookings
            .iter()
            .filter(|booking| booking.transfer_role(own_accounts).is_none())
            .filter(|booking| booking.kind() == BookingKind::Fee)
            .map(|booking| (booking.ordering_account().clone(), booking)),
    )
    .into_iter()
    .map(|((_month, account), flow)| Fee::of(account, flow))
    .collect()
}

/// Faltet Buchungen zu **Monats-Buckets** — die eine Bucketing-Mechanik, die sich die Sichten teilen:
/// jede Buchung fällt in den Monat ihres Buchungstags und wächst dort die Spalte ihrer Richtung.
///
/// Der `key` je Buchung ist, was die Sichten **zusätzlich** zum Monat trennt: nichts (`()`) beim
/// Cashflow eines Fensters, die Rolle des Gegenkontos bei den Umbuchungen, das belastete Konto bei
/// den Gebühren. Er steht im Schlüssel **hinter** dem Monat, sodass die `BTreeMap` zuerst
/// chronologisch ordnet und erst danach nach ihm — dieselbe stabile, eingabe-unabhängige Reihenfolge
/// für alle. Monate **ohne** Buchung entstehen gar nicht erst.
pub(crate) fn bucket_by_month<'a, K: Ord>(
    bookings: impl Iterator<Item = (K, &'a WindowedBooking)>,
) -> BTreeMap<(Month, K), MonthlyCashflow> {
    bookings.fold(BTreeMap::new(), |mut buckets, (key, booking)| {
        let month = Month::of(booking.day());
        let bucket = buckets
            .entry((month, key))
            .or_insert_with(|| MonthlyCashflow::empty(month));
        *bucket = bucket.plus(booking.amount(), booking.direction());
        buckets
    })
}

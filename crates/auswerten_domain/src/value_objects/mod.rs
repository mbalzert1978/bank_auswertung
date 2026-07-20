//! Die Wert-Objekte des Slice. Der Text-Engpass [`NonEmptyString`] ist die eine validierende
//! Stelle; darauf baut die [`Currency`]. Das Geld tragen [`Amount`] (vorzeichenlos) und
//! [`Direction`] (das Vorzeichen), die Zeitachse [`DayStamp`] (sortierbarer Tag-Stamp) und
//! [`Month`] (der Bucketing-Zeitraum, den dieser Slice aus der Achse rekonstruiert). Der
//! [`MonthlyCashflow`] ist der Bucket selbst, [`WindowedBooking`] die gelesene Buchung des
//! Fensters. Die [`WindowId`] ist die opake Identität des Aggregats, per Zeiger hereingereicht.
//!
//! Die Aggregate (`AnalysisWindow`, `WindowEvaluation`) sind über ihre Identität definiert und
//! liegen in `entities/`.

mod amount;
mod counter_account;
mod currency;
mod fingerprint;
mod month;
mod monthly_cashflow;
mod non_empty_string;
mod own_accounts;
mod recipient;
mod seed_marker;
mod window_id;
mod windowed_booking;

pub use amount::{Amount, Direction};
pub use counter_account::{AccountRole, AccountToken, CounterAccount};
pub use currency::Currency;
pub use fingerprint::Fingerprint;
pub use month::{DayStamp, Month};
pub use monthly_cashflow::MonthlyCashflow;
pub use non_empty_string::NonEmptyString;
pub use own_accounts::OwnAccounts;
pub use recipient::Recipient;
pub use seed_marker::SeedMarker;
pub use window_id::WindowId;
pub use windowed_booking::WindowedBooking;

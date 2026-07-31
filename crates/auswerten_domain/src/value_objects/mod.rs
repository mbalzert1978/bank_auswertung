//! Die Wert-Objekte des Slice. Der Text-Engpass [`NonEmptyString`] ist die eine validierende
//! Stelle; darauf baut die [`Currency`]. Das Geld tragen [`Amount`] (vorzeichenlos) und
//! [`Direction`] (das Vorzeichen), die Zeitachse [`DayStamp`] (sortierbarer Tag-Stamp) und
//! [`Month`] (der Bucketing-Zeitraum, den dieser Slice aus der Achse rekonstruiert). Der
//! [`MonthlyCashflow`] ist der Bucket selbst, [`WindowedBooking`] die gelesene Buchung des
//! Fensters. Die [`WindowId`] ist die opake Identität des Aggregats, per Zeiger hereingereicht.
//!
//! Der [`InternalTransfer`] ist die **andere Seite** des Cashflows: die Umbuchung zwischen eigenen
//! Konten, die aus ihm herausgenettet wurde und hier sichtbar bleibt.
//!
//! Das **Kategorisierer-Orakel** bringt drei mit: die [`Category`] als vergebenes Etikett und die
//! beiden schmalen DTOs seiner zwei Richtungen — [`CategoryRequest`] hinaus, [`CategoryAnswer`]
//! herein (ADR-0002: je Richtung ein eigenes, nie ein gemeinsames God-DTO).
//!
//! Die **fenster-übergreifende** Reichweite bringt zwei weitere mit: die [`RunSpan`] (die
//! Lauf-Spanne, die ihre Query gegen frühere Läufe abgrenzt) und das [`Occurrence`] (ein Vorkommen
//! einer Buchungsart auf der Zeitachse des Laufs).
//!
//! Die Aggregate (`AnalysisWindow`, `WindowEvaluation`) sind über ihre Identität definiert und
//! liegen in `entities/`.

mod amount;
mod category;
mod category_answer;
mod category_request;
mod counter_account;
mod currency;
mod fingerprint;
mod internal_transfer;
mod month;
mod monthly_cashflow;
mod non_empty_string;
mod occurrence;
mod own_accounts;
mod recipient;
mod run_span;
mod seed_marker;
mod window_id;
mod windowed_booking;

pub use amount::{Amount, Direction};
pub use category::Category;
pub use category_answer::CategoryAnswer;
pub use category_request::CategoryRequest;
pub use counter_account::{AccountRole, AccountToken, CounterAccount};
pub use currency::Currency;
pub use fingerprint::Fingerprint;
pub use internal_transfer::InternalTransfer;
pub use month::{DayStamp, Month};
pub use monthly_cashflow::MonthlyCashflow;
pub use non_empty_string::NonEmptyString;
pub use occurrence::Occurrence;
pub use own_accounts::OwnAccounts;
pub use recipient::Recipient;
pub use run_span::RunSpan;
pub use seed_marker::SeedMarker;
pub use window_id::WindowId;
pub use windowed_booking::WindowedBooking;

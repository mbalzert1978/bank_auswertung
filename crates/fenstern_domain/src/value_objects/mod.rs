//! Die Wert-Objekte des Slice. Der Text-Engpass [`NonEmptyString`] ist die eine validierende
//! Stelle; darauf baut die [`Currency`]. Die Zeitachse tragen [`DayStamp`] (sortierbarer Tag-Stamp,
//! ADR-0013 Entscheidung C), [`WindowDays`] (Seitengröße, ≥ 1 im Typ) und [`WindowSpan`] (die
//! wohlgeordnete Seite). Die Identitäten [`BookingId`] (Fremdschlüssel auf die obfuskierte Zeile)
//! und [`WindowId`] (die von der Ablage vergebene eigene ID) sind opak. Das Aggregat selbst
//! (`AnalysisWindow`) ist über seine Identität definiert und liegt in `entities/`.

mod booking_id;
mod currency;
mod non_empty_string;
mod window_id;
mod window_span;

pub use booking_id::BookingId;
pub use currency::Currency;
pub use non_empty_string::NonEmptyString;
pub use window_id::WindowId;
pub use window_span::{DayStamp, WindowDays, WindowSpan};

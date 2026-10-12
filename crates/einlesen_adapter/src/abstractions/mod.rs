//! Die öffentlichen, mechanischen Seams, die **IO** erfüllt: roh-typisiert, ohne
//! Domänen-Kenntnis. Der Handler kennt sie **nicht** — er hängt nur an den internen
//! Domänen-Ports (`einlesen_domain`); die Port-Bridges (z. B.
//! [`crate::adapter::CalendarBridge`]) mappen zwischen beiden Welten.

mod calendar_seam;
mod currency_seam;
mod money_seam;
mod publish_seam;
mod source_seam;
mod store_seam;

pub use calendar_seam::{CalendarDay, CalendarSeam};
pub use currency_seam::{CurrencySeam, ResolvedCurrency};
pub use money_seam::{DecodedAmount, MoneySeam, Sign};
pub use publish_seam::{PublishSeam, PublishUnavailable};
pub use source_seam::{SourceSeam, SourceUnavailable};
pub use store_seam::{StoreOutcome, StoreSeam, StoreUnavailable};

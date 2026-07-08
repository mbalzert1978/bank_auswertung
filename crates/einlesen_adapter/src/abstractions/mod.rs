//! Die öffentlichen, mechanischen Seams, die **IO** erfüllt (Vorbild
//! `examples/Downloader/Adapter/Abstractions`): roh-typisiert, ohne Domänen-Kenntnis. Der
//! Handler kennt sie **nicht** — er hängt nur an den internen Domänen-Ports
//! (`einlesen_domain`); die Port-Bridges (z. B. [`crate::bridges::CalendarBridge`]) mappen
//! zwischen beiden Welten.

mod calendar_seam;
mod store_seam;

pub use calendar_seam::{CalendarDay, CalendarSeam};
pub use store_seam::StoreSeam;

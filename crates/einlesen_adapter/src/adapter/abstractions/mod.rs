//! Die öffentlichen, mechanischen Seams, die **IO** erfüllt (Vorbild
//! `examples/Downloader/Adapter/Abstractions`): roh-typisiert, ohne Domänen-Kenntnis. Der
//! Handler kennt sie **nicht** — er hängt nur an den internen Domänen-Ports
//! (`einlesen_domain`); die Port-Bridges (z. B. [`crate::kalender_bridge`]) mappen zwischen
//! beiden Welten.

mod kalender_seam;

pub use kalender_seam::{KalenderSeam, Kalendertag};

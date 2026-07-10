//! Die DTOs des Slice: das eingehende Seam-DTO [`RawRecord`] (außen → innen, über den
//! Quell-Seam beschafft) und das ausgehende [`Line`] samt adapter-eigener [`Direction`]
//! (innen → außen). Beide tragen nur primitive/adapter-eigene Typen — kein Domänentyp
//! überquert die Adapter-Grenze.

mod line;
mod raw_record;

pub use line::{Currency, Direction, Line};
pub use raw_record::RawRecord;

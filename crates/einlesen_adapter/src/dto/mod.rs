//! Die DTOs des Slice: das eingehende Wire-DTO [`RawRecord`] (außen → innen) und das
//! ausgehende [`Line`] samt adapter-eigener [`Direction`] (innen → außen). Beide tragen nur
//! primitive/adapter-eigene Typen — kein Domänentyp überquert die Adapter-Grenze.

mod line;
mod raw_record;

pub use line::{Currency, Direction, Line};
pub(crate) use raw_record::MissingMandatoryField;
pub use raw_record::RawRecord;

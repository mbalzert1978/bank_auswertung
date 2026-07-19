//! Die **Entities** des Slice: das über seine Identität definierte Aggregat [`AnalysisWindow`] (in
//! eigener Ablage abgelegt, ID von der Persistenz vergeben) und die [`WindowedBooking`], die der
//! Lese-Port als Zeile der Seite liefert und die das Fenster nur per Fremdschlüssel einfriert.

mod analysis_window;
mod windowed_booking;

pub use analysis_window::AnalysisWindow;
pub use windowed_booking::WindowedBooking;

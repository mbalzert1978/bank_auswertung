/// Ob die Gegenseite einer Buchung eines der eigenen Konten ist (`Eigen`,
/// dann eine `Umbuchung`) oder nicht (`Fremd`, echter Zu-/Abfluss).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gegenkonto {
    Eigen,
    Fremd,
}

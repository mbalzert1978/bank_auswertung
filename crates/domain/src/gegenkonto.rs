use crate::Rolle;

/// Ob die Gegenseite einer Buchung eines der eigenen Konten ist (`Eigen`, dann
/// eine `Umbuchung`) oder nicht (`Fremd`, echter Zu-/Abfluss).
///
/// Bei `Eigen` trägt der Typ die `Konto-Rolle` des getroffenen eigenen Kontos —
/// „eigen ohne Rolle" ist damit unkonstruierbar. Nach der Obfuskierung ist die
/// Gegen-IBAN unwiederbringlich weg; die Rolle muss also schon beim Mappen
/// mitkommen, sonst könnte die Analyse (Aderlass je Konto-Rolle) sie nicht mehr
/// rekonstruieren. Welche Gegen-IBAN eigen ist und mit welcher Rolle, löst der
/// eingehende Adapter über die Eigene-Konten-Config auf; die Domäne erhält das
/// Ergebnis bereits obfuskiert.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gegenkonto {
    Eigen(Rolle),
    Fremd,
}

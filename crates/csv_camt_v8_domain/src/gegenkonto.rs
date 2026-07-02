use crate::rolle::RolleEntity;

/// Ob die Gegenseite einer Buchung eines der eigenen Konten ist (`Eigen`, dann
/// eine `Umbuchung`) oder nicht (`Fremd`, echter Zu-/Abfluss).
///
/// Bei `Eigen` trägt der Typ die `Konto-Rolle` des getroffenen eigenen Kontos —
/// „eigen ohne Rolle" ist damit unkonstruierbar. Nach der Obfuskierung ist die
/// Gegen-IBAN unwiederbringlich weg; die Rolle muss also schon beim Mappen
/// mitkommen, sonst könnte die Analyse (Aderlass je Konto-Rolle) sie nicht mehr
/// rekonstruieren. Welche Gegen-IBAN eigen ist und mit welcher Rolle, löst der
/// Adapter über den Eigene-Konten-Slice auf; die Domäne erhält das Ergebnis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GegenkontoEntity {
    Eigen(RolleEntity),
    Fremd,
}

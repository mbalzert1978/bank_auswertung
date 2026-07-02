/// Die Richtung einer Buchung. Trägt das Vorzeichen — `BetragEntity` selbst ist
/// vorzeichenlos, eine negative `Eingang`-Buchung ist damit unkonstruierbar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RichtungEntity {
    Eingang,
    Ausgang,
}

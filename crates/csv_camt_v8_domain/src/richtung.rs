/// Die Richtung einer Buchung. Trägt das Vorzeichen — `BetragEntity` selbst ist
/// vorzeichenlos, eine negative `Eingang`-Buchung ist damit unkonstruierbar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RichtungEntity {
    Eingang,
    Ausgang,
}

impl RichtungEntity {
    /// Deutet das Vorzeichen eines Betrags: `-` → `Ausgang`, `+` bzw. kein
    /// Vorzeichen → `Eingang`. Liefert die Richtung und den vom Vorzeichen
    /// befreiten Ziffernteil. So lebt die Vorzeichen-Deutung bei der Richtung,
    /// nicht beim Betrag.
    pub(crate) fn aus_vorzeichen(betrag: &str) -> (Self, &str) {
        match betrag.strip_prefix('-') {
            Some(rest) => (Self::Ausgang, rest),
            None => (Self::Eingang, betrag.strip_prefix('+').unwrap_or(betrag)),
        }
    }
}

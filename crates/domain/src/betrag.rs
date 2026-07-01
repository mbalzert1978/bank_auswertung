/// Die Währung eines `Betrag`s. Die Domäne summiert keine gemischten
/// Währungen — der eingehende Adapter sichert eine einheitliche Währung zu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Währung {
    Eur,
}

/// Ein vorzeichenloser Geldbetrag mit Währung. Das Vorzeichen lebt
/// ausschließlich in `Richtung::Eingang|Ausgang`, nie hier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Betrag {
    cent: u64,
    währung: Währung,
}

impl Betrag {
    /// Liefert `None` bei negativem `cent` — ein negativer Betrag ist nicht
    /// konstruierbar.
    pub fn new(cent: i64, währung: Währung) -> Option<Self> {
        u64::try_from(cent).ok().map(|cent| Self { cent, währung })
    }

    pub fn cent(&self) -> u64 {
        self.cent
    }

    pub fn währung(&self) -> Währung {
        self.währung
    }
}

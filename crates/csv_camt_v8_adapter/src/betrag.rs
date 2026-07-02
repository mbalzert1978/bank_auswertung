use csv_camt_v8_domain::{BetragEntity, WährungEntity};

/// Das UBQL-DTO für die Währung eines Betrags am Slice-Rand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Währung {
    Eur,
}

impl Währung {
    pub(crate) fn entity(self) -> WährungEntity {
        match self {
            Self::Eur => WährungEntity::Eur,
        }
    }

    pub(crate) fn from_entity(entity: WährungEntity) -> Self {
        match entity {
            WährungEntity::Eur => Self::Eur,
        }
    }
}

/// Das UBQL-DTO für einen vorzeichenlosen Geldbetrag mit Währung — der **Ausgang**
/// (Teil des `Buchung`-DTOs). Das Vorzeichen lebt in [`Richtung`](crate::Richtung),
/// nie hier. Reiner Carrier: umschließt den Domänen-Wert; das Deuten des deutschen
/// Betragstexts liegt in der Domäne ([`BetragEntity::aus_deutschem_betrag`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Betrag(BetragEntity);

impl Betrag {
    /// Baut das DTO aus Cent + Währung (etwa zum Aufbau erwarteter Werte). Ein
    /// `u64`-Cent ist konstruktionsbedingt nicht-negativ.
    pub fn new(cent: u64, währung: Währung) -> Self {
        let cent = i64::try_from(cent).expect("Cent-Betrag passt in i64");
        Self(
            BetragEntity::new(cent, währung.entity())
                .expect("nicht-negativer Betrag ist konstruierbar"),
        )
    }

    /// Projiziert den Domänen-Wert auf das DTO — crate-intern (Projektion).
    pub(crate) fn from_entity(entity: BetragEntity) -> Self {
        Self(entity)
    }

    pub fn cent(&self) -> u64 {
        self.0.cent()
    }

    pub fn währung(&self) -> Währung {
        Währung::from_entity(self.0.währung())
    }
}

/// Die eine Stelle, an der „was ist derselbe Währungscode" entschieden wird:
/// getrimmt und in Großbuchstaben. Sowohl die Einheitlichkeitsprüfung als auch das
/// Mapping auf die Domänen-`WährungEntity` gehen hierdurch, damit sie nicht
/// auseinanderlaufen.
pub(crate) fn währung_normalized(raw: &str) -> String {
    raw.trim().to_uppercase()
}

/// Mappt einen **bereits normalisierten** Währungscode auf die Domänen-Währung.
/// Nur `EUR` ist bekannt; alles andere ist `None` (die Domäne summiert keine
/// fremde Währung). Trivialer Nachschlag, kein Format-Algorithmus.
pub(crate) fn währung_from_str(normalized: &str) -> Option<WährungEntity> {
    match normalized {
        "EUR" => Some(WährungEntity::Eur),
        _ => None,
    }
}

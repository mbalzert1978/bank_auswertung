use obfuskierung_domain::{SpeicherError, ZuordnungsSpeicher, Zuordnungsbestand};

use crate::obfuskierer::Obfuskierer;

/// Fluent-TestApi des Obfuskierungs-Slice ([ADR-0002](../../../docs/adr/0002-dto-grenze-und-anti-corruption-layer.md)):
/// hält einen geteilten In-Memory-Store (spielt die vertagte Persistenz) und
/// gibt darüber geladene [`Obfuskierer`] heraus. So lässt sich die
/// laufübergreifende Stabilität als Black-Box prüfen: obfuskieren, [`sichern`](Self::sichern),
/// einen frischen [`Obfuskierer`] holen und dieselben Token wiedersehen — alles
/// über die public DTOs, ohne Interna zu berühren.
#[derive(Debug, Default)]
pub struct ObfuskierungTestApi {
    speicher: FakeSpeicher,
}

impl ObfuskierungTestApi {
    /// Neuer, leerer Store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Ein über dem geteilten Store geladener Obfuskierer (erbt dessen bisherigen
    /// Bestand — leer beim ersten Aufruf).
    pub fn obfuskierer(&self) -> Obfuskierer {
        Obfuskierer::laden(&self.speicher).expect("In-Memory-Store lädt immer")
    }

    /// Sichert den Bestand eines Obfuskierers in den geteilten Store zurück, sodass
    /// ein danach geholter [`Obfuskierer`] die Token laufübergreifend fortführt.
    pub fn sichern(&mut self, obfuskierer: &Obfuskierer) {
        obfuskierer
            .sichern(&mut self.speicher)
            .expect("In-Memory-Store sichert immer");
    }
}

/// Fake-Store: hält den Bestand im Speicher (persistiert nichts auf Platte) und
/// spielt so die vertagte Persistenz für die Specs. Innerhalb einer TestApi
/// laufübergreifend stabil.
#[derive(Debug, Default)]
struct FakeSpeicher {
    bestand: Zuordnungsbestand,
}

impl ZuordnungsSpeicher for FakeSpeicher {
    fn laden(&self) -> Result<Zuordnungsbestand, SpeicherError> {
        Ok(self.bestand.clone())
    }

    fn sichern(&mut self, bestand: &Zuordnungsbestand) -> Result<(), SpeicherError> {
        self.bestand = bestand.clone();
        Ok(())
    }
}

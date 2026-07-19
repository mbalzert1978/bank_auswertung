use std::collections::BTreeMap;

use crate::abstractions::{ObfuscatedWindowSeam, SourceUnavailable};
use crate::models::ObfuscatedRow;

/// Fake des Lese-Seams: hält die arrangierten obfuskierten Zeilen **je Buchungstag** im Speicher und
/// spielt so die vertagte `obfuskiert`-Ablage für die Specs. `read` liefert genau die Zeilen, deren
/// Tag in der Spanne `[from … to]` liegt (beide Grenzen einschließlich) — die Seiten-Query, auf der
/// die Seitengröße überhaupt beobachtbar wird. Die `BTreeMap` hält die Lesereihenfolge stabil
/// (nach Tag aufsteigend), damit ein Spec die eingefrorenen Fremdschlüssel exakt erwarten kann.
///
/// Mit [`fail_with`](Self::fail_with) spielt er stattdessen den terminalen Beschaffungs-Fehlschlag.
#[derive(Default)]
pub(crate) struct FakeObfuscatedWindowSource {
    rows: BTreeMap<i64, Vec<ObfuscatedRow>>,
    failure: Option<String>,
}

impl FakeObfuscatedWindowSource {
    /// Arrangiert eine obfuskierte Zeile unter ihrem Buchungstag.
    pub(crate) fn insert(&mut self, booking_day: i64, row: ObfuscatedRow) {
        self.rows.entry(booking_day).or_default().push(row);
    }

    /// Lässt jede Beschaffung terminal mit der gegebenen io-Diagnose scheitern.
    pub(crate) fn fail_with(&mut self, message: &str) {
        self.failure = Some(message.to_string());
    }
}

impl ObfuscatedWindowSeam for FakeObfuscatedWindowSource {
    fn read(&self, from: i64, to: i64) -> Result<Vec<ObfuscatedRow>, SourceUnavailable> {
        self.failure
            .clone()
            .map_or(Ok(()), |message| Err(SourceUnavailable { message }))?;
        Ok(self
            .rows
            .range(from..=to)
            .flat_map(|(_, rows)| rows.iter().cloned())
            .collect())
    }
}

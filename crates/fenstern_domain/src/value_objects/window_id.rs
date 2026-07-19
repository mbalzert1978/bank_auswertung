/// Die **eigene** Identität eines abgelegten [`AnalysisWindow`](crate::AnalysisWindow) — die
/// **Ablage vergibt sie** (io: `INSERT … RETURNING id`), und **genau sie** publiziert der Slice als
/// Zeiger nach `auswerten` (uniformes ID-Handoff, ADR-0012: Tabelle = Wahrheit, Message = Pointer).
/// Opak; der Slice deutet sie nie.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WindowId(String);

impl WindowId {
    /// Baut die ID aus dem Rohwert, den der Speicher-Seam beim Ablegen zurückgibt. Bewusst **opak**
    /// (kein `NonEmptyString`, keine fehlbare Signatur): die ID vergibt die Persistenz, der Slice
    /// deutet sie nie, er reicht sie als Zeiger durch. Die **Nicht-Leere** ist dennoch abgesichert:
    /// die Ablage garantiert eine Identität, eine leere ID ist ein **unmöglicher** Zustand — ein
    /// Herkunfts-Invariantenbruch. Genau darum fällt der Guard **hart** um (`panic`) statt einen
    /// `Result`-Pfad für einen nie eintretenden Fall aufzuspannen; ein leer publizierter Zeiger
    /// würde sonst still nach `auswerten` durchlaufen. (Spiegelt `einlesen_domain::IngestionId::new`
    /// und die gleichnamige ID des Obfuskierungs-Slice. Der [`BookingId`](crate::BookingId) dieses
    /// Slice braucht den Guard **nicht** — sie entsteht nur aus einem bereits geprüften
    /// `NonEmptyString`.)
    ///
    /// # Panics
    /// Wenn der Rohwert leer ist — die Ablage hat ihre Garantie (nie-leere Identität) gebrochen.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        let value = value.into();
        assert!(
            !value.is_empty(),
            "WindowId: leere ID — die Ablage garantiert eine nie-leere Identität, ein Leerwert ist \
             ein Invariantenbruch"
        );
        Self(value)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

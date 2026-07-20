/// Die **Identität des eingefrorenen Auswertungsfensters**, auf die das konsumierte
/// `FensterGeschlossen` zeigt — von `fenstern`s Ablage vergeben und als **Zeiger** über die
/// Nachricht gereicht (uniformes ID-Handoff, ADR-0012: Tabelle = Wahrheit, Message = Pointer). Der
/// Handler bekommt **nur sie**, nie das eingebettete Aggregat.
///
/// Opak: der Slice deutet sie nie, er liest damit und hält sie als Fremdschlüssel am Ergebnis fest.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WindowId(String);

impl WindowId {
    /// Baut die ID aus dem Rohwert der Envelope. Bewusst **opak** (kein `NonEmptyString`, keine
    /// fehlbare Signatur): sie stammt aus der Ablage des Vorgänger-Slice, der Slice deutet sie nie.
    /// Die **Nicht-Leere** ist dennoch abgesichert — eine leere ID ist ein **unmöglicher** Zustand
    /// (ein Herkunfts-Invariantenbruch: `fenstern`s Ablage garantiert eine nie-leere Identität).
    /// Genau darum fällt der Guard **hart** um, statt einen `Result`-Pfad für einen nie eintretenden
    /// Fall aufzuspannen; der Decoder weist eine leere Envelope ohnehin schon vorher ab.
    /// (Spiegelt `fenstern_domain::WindowId::new` und `einlesen_domain::IngestionId::new`.)
    ///
    /// # Panics
    /// Wenn der Rohwert leer ist — der Vorgänger hat seine Garantie (nie-leere Identität) gebrochen.
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

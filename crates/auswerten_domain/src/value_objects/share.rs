use crate::value_objects::Amount;

/// Wie viele Basispunkte das **Ganze** sind: 10 000 — ein Basispunkt ist ein Hundertstel Prozent.
const WHOLE_BASIS_POINTS: u128 = 10_000;

/// Ein **Anteil am Ganzen** in Basispunkten (1/100 Prozent, das Ganze sind 10 000) — die Zahl, mit
/// der das Pareto-Ranking sagt, wie schwer ein Kostentreiber wiegt
/// ([Issue 0007](../../../../docs/issues/0007-aderlass-pareto-ranking-und-spar-kandidaten.md)).
///
/// **Ganzzahlig, abgerundet, ohne Gleitkomma:** ein `f64`-Anteil rundete je nach Reihenfolge der
/// Summanden anders, und „sind das genau 100 %?" wäre keine sichere Frage mehr. Basispunkte sind
/// fein genug für einen Bericht (12,34 %) und vergleichen sich exakt.
///
/// **Ohne Rundungsrest im Ranking:** abgerundet wird nur der kumulierte Anteil; ein Einzelanteil ist
/// die Differenz zweier kumulierter (`minus`). So ergeben die Anteile aller Plätze
/// genau 10 000 — getrennt abgerundet verlöre jeder Platz bis zu einen Basispunkt.
///
/// **Nie über dem Ganzen, per Konstruktion:** der einzige Konstruktor kappt den Teil am Ganzen, ein
/// Anteil über 100 % ist damit unkonstruierbar, nicht „geprüft" (ADR-0005).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Share(u32);

impl Share {
    /// Der Anteil von `part` an `whole`, auf ganze Basispunkte **abgerundet**. Ein leeres Ganzes
    /// (`whole = 0`) hat keine Teile — der Anteil ist dann null, nicht „undefiniert".
    ///
    /// Gerechnet in `u128`, damit das Produkt mit 10 000 nie überläuft; das Ergebnis liegt per
    /// Kappung in `0..=10_000` und passt darum immer in `u32` (der Rückfall auf das Ganze ist der
    /// unerreichbare Zweig, nicht ein stiller Fehler).
    #[must_use]
    pub(crate) fn of(part: Amount, whole: Amount) -> Self {
        let whole = u128::from(whole.minor_units());
        let part = u128::from(part.minor_units()).min(whole);
        let basis_points = part
            .saturating_mul(WHOLE_BASIS_POINTS)
            .checked_div(whole)
            .unwrap_or(0);
        Self(u32::try_from(basis_points).unwrap_or(10_000))
    }

    /// Der Abstand zweier Anteile — der Einzelanteil eines Platzes als Differenz seines kumulierten
    /// Anteils zu dem des Vorplatzes. **Sättigend:** kumulierte Anteile wachsen monoton, ein
    /// negativer Abstand ist darum unerreichbar und fiele auf null statt zu überlaufen.
    #[must_use]
    pub(crate) fn minus(self, earlier: Self) -> Self {
        Self(self.0.saturating_sub(earlier.0))
    }

    /// Der Anteil in Basispunkten, `0..=10_000`.
    #[must_use]
    pub fn basis_points(self) -> u32 {
        self.0
    }
}

use crate::value_objects::NonEmptyString;

/// Das **opake Kennzeichen einer Tokenisierung** — nicht der Seed, sondern ein nicht umkehrbares
/// Etikett seiner Identität, das io sowohl dem eingefrorenen Fenster als auch der
/// Eigene-Konten-Config mitgibt.
///
/// **Warum es überhaupt existiert.** Die Auflösung Eigen/Fremd vergleicht Token gegen Token. Wären
/// die beiden Seiten unter **verschiedenen** Seeds tokenisiert, matchte kein einziges Token — und
/// jede Umbuchung liefe **lautlos** als „fremd" durch. Das Ergebnis wiese Umbuchungen als Konsum
/// aus, ohne dass irgendetwas fehlschlüge: ein stiller Datenfehler, und die teuerste Sorte. Der
/// Marker macht die Voraussetzung, auf der der Vergleich ruht, **prüfbar** statt bloß angenommen.
///
/// **Warum nicht der Seed selbst.** Der Seed ist das lokale Geheimnis
/// ([ADR-0010](../../../../docs/adr/0010-seed-basierte-pseudonym-stabilitaet.md)); er erreicht
/// diesen Slice nie — auch nicht „nur zum Vergleichen". Für die Frage „stammen beide Seiten aus
/// derselben Tokenisierung?" genügt ein Etikett, das Gleichheit trägt und sonst nichts.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SeedMarker(NonEmptyString);

impl From<NonEmptyString> for SeedMarker {
    fn from(value: NonEmptyString) -> Self {
        Self(value)
    }
}

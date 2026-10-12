use crate::value_objects::DayStamp;

/// Die **Lauf-Spanne** — `[from … to]`, beide Grenzen einschließlich: die Zeitachse **genau eines**
/// Laufs, die `fenstern` beim Versiegen der Historie mit `HistorieVollständig` mitträgt (ADR-0013,
/// Entscheidung E).
///
/// Sie begrenzt die Zeitachse der fenster-übergreifenden Query. **Gegen frühere Läufe grenzt sie
/// allein nicht ab:** jeder Lauf paginiert dieselbe Historie, die Fenster eines früheren Laufs liegen
/// in derselben Spanne. Den Lauf selbst legt die Lese-Bridge des Adapters als Scope an den Seam
/// ([ADR-0016](../../../docs/adr/0016-zeilen-schluessel-und-lauf-eigene-fenster.md)); die Domäne
/// bleibt lauf-blind. Ohne diesen Scope zöge die Query die Aggregate früherer Läufe still mit herein.
///
/// **Unfehlbar und wohlgeordnet:** sie ordnet ihre beiden Eckpunkte selbst (`from = min`,
/// `to = max`), statt eine verdrehte Eingabe „zu prüfen". Eine verdrehte Spanne ist damit
/// unkonstruierbar, nicht geprüft (ADR-0005) — es gibt keinen Fehler-Ausgang und keinen illegalen
/// Zustand, den ein Aufrufer erzeugen könnte.
///
/// **Sie prüft selbst keine Zugehörigkeit.** Ob ein Tag in der Spanne liegt, entscheidet **io** als
/// `WHERE`-Klausel — die Domäne reicht die Spanne an den Lese-Port und bekommt zurück, was in ihr
/// liegt. Ein `contains` hier wäre eine zweite, divergierbare Kopie derselben Regel, die niemand
/// aufriefe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunSpan {
    from: DayStamp,
    to: DayStamp,
}

impl RunSpan {
    /// Die Spanne über zwei Eckpunkte, in beliebiger Reihenfolge hereingereicht.
    #[must_use]
    pub fn spanning(a: DayStamp, b: DayStamp) -> Self {
        Self {
            from: a.min(b),
            to: a.max(b),
        }
    }

    #[must_use]
    pub fn from(self) -> DayStamp {
        self.from
    }

    #[must_use]
    pub fn to(self) -> DayStamp {
        self.to
    }
}

use crate::value_objects::DayStamp;

/// Die **Lauf-Spanne** — `[from … to]`, beide Grenzen einschließlich: die Zeitachse **genau eines**
/// Laufs, die `fenstern` beim Versiegen der Historie mit `HistorieVollständig` mitträgt (ADR-0013,
/// Entscheidung E).
///
/// Sie ist der **Scope** der fenster-übergreifenden Query, und das ist ihr ganzer Zweck: ohne sie
/// zöge die Query auch die Aggregate **früherer** Läufe herein, und der Rhythmus, den 0005 darauf
/// erkennt, wäre der einer Zeitachse, die es nie gab. Der Fehler wäre dabei **still** — es käme ein
/// Ergebnis heraus, nur das falsche.
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

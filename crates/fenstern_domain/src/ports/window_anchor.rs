use crate::error::WindowError;
use crate::value_objects::DayStamp;

/// Der von der Domäne **verlangte** Anker-Port (ADR-0013, Entscheidung D): er beantwortet die
/// Anker-Frage über dem obfuskierten Bestand. `fenstern` nutzt ihn an **beiden** Enden eines Laufs
/// — es ist **eine** Query-Art, einmal ohne und einmal mit Untergrenze:
///
/// - [`latest`](Self::latest) (`max(booking_day)`) liefert dem **Anstoß** den Initial-Anker: der
///   Lauf startet **datenrelativ** am jüngsten Buchungstag des Bestands, nicht an einer Uhr — darum
///   ist er deterministisch und in den Specs ohne Zeitquelle arrangierbar;
/// - [`latest_before`](Self::latest_before) (`max(booking_day < from)`) trägt die
///   **Selbst-Fortsetzung**: der nächste Anker ist der jüngste echte Buchungstag unter der
///   Fensterunterkante.
///
/// **Ein** Mechanismus trägt damit Fortsetzung **und** Terminierung: liefert er einen Tag, geht die
/// Kette weiter; liefert er `None`, ist die Historie erschöpft. Kein Zähler, kein Boolean, kein
/// Watermark-Sonderfall (ADR-0013, Entscheidung B, Präzisierung 2). Beim **Anstoß** ist dasselbe
/// `None` der **leere Datenbestand**: der Lauf endet dort mit **Erfolg** — Erschöpfung schon bei
/// Schritt 0, symmetrisch zum normalen Ketten-Ende.
///
/// **Fehlbar und terminal** ([`WindowError`]): ein Fehlschlag der Abfrage ist ein Fehler, `Ok(None)`
/// dagegen die **gültige** Antwort „darunter liegt nichts mehr" — darum `Result<Option<_>>` und
/// nicht ein überladenes `Option`.
pub trait WindowAnchor {
    /// Der **jüngste Buchungstag des ganzen Bestands** (`max(booking_day)`) — der Initial-Anker des
    /// Anstoßes. `Ok(None)` heißt **leerer Bestand**: eine gültige Antwort, kein Fehler.
    fn latest(&self) -> Result<Option<DayStamp>, WindowError>;

    /// Der jüngste Buchungstag **echt unterhalb** von `day`, oder `None`, wenn keiner mehr existiert.
    fn latest_before(&self, day: DayStamp) -> Result<Option<DayStamp>, WindowError>;
}

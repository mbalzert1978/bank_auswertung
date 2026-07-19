use crate::error::WindowError;
use crate::value_objects::DayStamp;

/// Der von der Domäne **verlangte** Anker-Port (ADR-0013, Entscheidung D): er beantwortet die
/// Anker-Frage über dem obfuskierten Bestand. `fenstern` nutzt ihn für die **Selbst-Fortsetzung**:
/// der nächste Anker ist der **jüngste echte Buchungstag unter der Fensterunterkante**
/// (`max(booking_day < from)`) — dieselbe Query-Art wie der Initial-Anker, nur mit Untergrenze.
///
/// **Ein** Mechanismus trägt damit Fortsetzung **und** Terminierung: liefert er einen Tag, geht die
/// Kette weiter; liefert er `None`, ist die Historie erschöpft. Kein Zähler, kein Boolean, kein
/// Watermark-Sonderfall (ADR-0013, Entscheidung B, Präzisierung 2).
///
/// **Fehlbar und terminal** ([`WindowError`]): ein Fehlschlag der Abfrage ist ein Fehler, `Ok(None)`
/// dagegen die **gültige** Antwort „darunter liegt nichts mehr" — darum `Result<Option<_>>` und
/// nicht ein überladenes `Option`.
pub trait WindowAnchor {
    /// Der jüngste Buchungstag **echt unterhalb** von `day`, oder `None`, wenn keiner mehr existiert.
    fn latest_before(&self, day: DayStamp) -> Result<Option<DayStamp>, WindowError>;
}

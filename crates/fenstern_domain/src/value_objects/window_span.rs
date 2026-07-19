use std::num::NonZeroU32;

/// Ein Buchungstag als **sortierbarer Tag-Stamp** (Tage seit Epoche, feste Zone —
/// [ADR-0013](../../../docs/adr/0013-zeitfenster-collector-paginierender-analyse-kopf.md),
/// Entscheidung C). Bewusst ein Integer und **kein** Kalender-Typ: die Domäne ist dependency-frei,
/// und `fenstern` rechnet auf der Achse nur `± Tage` und vergleicht — die Kalender-Sicht
/// (Monats-Bucketing) rekonstruiert erst `auswerten`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DayStamp(i64);

impl DayStamp {
    #[must_use]
    pub fn new(value: i64) -> Self {
        Self(value)
    }

    #[must_use]
    pub fn value(self) -> i64 {
        self.0
    }
}

/// Die **Seitengröße des Pagers** in Tagen (real ~400 — bewusst > 365, damit ein voller
/// Jahreszyklus in **eine** Seite fällt). Sie kommt dem Handler als **Wert** herein: **io liest die
/// Config**, die Domäne liest keine Datei (ADR-0013, Entscheidung B — genau wie der Seed bei
/// `obfuskierung`).
///
/// **Nicht verwechseln** mit dem Berichts-/Bucketing-*Zeitraum* (CONTEXT.md, Markierte
/// Mehrdeutigkeit „Fenster vs. Zeitraum"): ein Fenster deckt viele Zeiträume ab.
///
/// Eine Seitengröße von **null** ist **unkonstruierbar** — der Typ trägt es
/// ([`NonZeroU32`]), es gibt keinen „geprüften" Pfad und keinen Fehler-Ausgang dafür.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowDays(NonZeroU32);

impl From<NonZeroU32> for WindowDays {
    fn from(days: NonZeroU32) -> Self {
        Self(days)
    }
}

/// Die **Spanne einer Seite** — `[from … to]`, beide Grenzen einschließlich. Sie ist **nur** über
/// [`WindowSpan::page`] konstruierbar und damit per Konstruktion wohlgeordnet (`from <= to`): eine
/// verdrehte oder entartete Spanne ist unkonstruierbar, nicht „geprüft" (ADR-0005).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowSpan {
    from: DayStamp,
    to: DayStamp,
}

impl WindowSpan {
    /// Die Seite zum Anker: `[anchor − window_days + 1 … anchor]` (ADR-0013, Entscheidung B). Der
    /// Anker ist die **obere** Grenze — der Pager läuft neu→alt.
    ///
    /// **Unfehlbar:** [`WindowDays`] ist ≥ 1, also ist der Abzug `window_days − 1` ≥ 0 und `from`
    /// nie größer als `to`; die Subtraktion sättigt, damit auch ein extremer Anker keinen Überlauf
    /// auslöst.
    #[must_use]
    pub fn page(anchor: DayStamp, window_days: WindowDays) -> Self {
        let reach = i64::from(window_days.0.get().saturating_sub(1));
        Self {
            from: DayStamp(anchor.0.saturating_sub(reach)),
            to: anchor,
        }
    }

    #[must_use]
    pub fn from(&self) -> DayStamp {
        self.from
    }

    #[must_use]
    pub fn to(&self) -> DayStamp {
        self.to
    }
}

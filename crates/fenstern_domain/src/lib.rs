//! fenstern-Slice — **Domäne** (blind, ohne Dependencies,
//! [ADR-0007](../../../docs/adr/0007-modularer-monolith-feature-slices.md) /
//! [ADR-0013](../../../docs/adr/0013-zeitfenster-collector-paginierender-analyse-kopf.md)).
//!
//! Der dritte Pipeline-Schritt (`einlesen → obfuskieren → fenstern → auswerten → report`) und der
//! **paginierende Analyse-Kopf**. Anders als seine Vorgänger ist er **N→1**: der
//! [`WindowingHandler`] spannt zum Anker des Kommandos die Seite
//! `[anchor − window_days + 1 … anchor]` auf, **liest** ihre obfuskierten Buchungen
//! ([`ObfuscatedWindowSource`]), **reduziert** sie zu **einem** [`AnalysisWindow`], **legt** es ab
//! ([`AnalysisWindowRepository`], das die neue ID vergibt) und **publiziert** diese neue ID als
//! Zeiger ([`WindowClosedPublisher`]). Lesen, Ablegen und Publizieren sind je fehlbar und terminal
//! ([`WindowError`]) — kein Dead-Letter, kein Retry (ADR-0012).
//!
//! Danach **setzt er sich selbst fort**: der nächste Anker ist der jüngste echte Buchungstag unter
//! der Fensterunterkante ([`WindowAnchor`], `max(booking_day < from)`). Liefert die Abfrage einen
//! Tag, publiziert er das nächste [`AdvanceWindow`] ([`AdvanceWindowPublisher`]); liefert sie
//! nichts, ist die Historie erschöpft und er publiziert den Abschluss mit der Lauf-Spanne
//! ([`HistoryCompletePublisher`]). **Ein** Mechanismus für Fortsetzung und Terminierung — kein
//! Zähler, kein Boolean, kein Watermark-Sonderfall (ADR-0013, Entscheidung B, Präzisierung 2).
//!
//! Fachlicher Kern: die **Materialisierung** des Fensters. Sie kauft Snapshot-Stabilität (welche
//! Zeilen zum Fenster gehören, friert ein), die Barriere als Daten und Determinismus bei
//! Redelivery. Die **Invarianten sitzen an der Konstruktion** von [`AnalysisWindow`] — **nicht
//! leer** und **eine Währung** sind unkonstruierbar zu verletzen, nicht „geprüft" (ADR-0005).
//!
//! Davor steht der **Anstoß**: der [`StartHandler`] konsumiert das **feldlose** [`StartAnalysis`],
//! liest den Initial-Anker **einmalig** über denselben [`WindowAnchor`]-Port (`max(booking_day)` —
//! datenrelativ, keine Uhr) und publiziert damit das erste [`AdvanceWindow`]. Zwei Kommandos, zwei
//! einzweckige Handler: der **Typ trägt die Absicht**, „leerer Anker" ist unkonstruierbar (ADR-0005).
//! Ein **leerer Bestand** ist dabei **kein Fehler** — der Lauf endet mit Erfolg, ohne jede Nachricht.
//!
//! Die Seitengröße [`WindowDays`] kommt als **Wert** herein (io liest die Config); die Domäne liest
//! keine Datei und kennt keine Config. Die Zeitachse ist ein sortierbarer [`DayStamp`] (ADR-0013,
//! Entscheidung C) — kein Kalender-Typ, denn die Domäne bindet nichts.

mod entities;
mod error;
mod handler;
mod models;
mod ports;
mod start_handler;
mod value_objects;

pub use entities::{AnalysisWindow, WindowedBooking};
pub use error::WindowError;
pub use handler::WindowingHandler;
pub use models::{AdvanceWindow, StartAnalysis};
pub use ports::{
    AdvanceWindowPublisher, AnalysisWindowRepository, HistoryCompletePublisher,
    ObfuscatedWindowSource, WindowAnchor, WindowClosedPublisher,
};
pub use start_handler::StartHandler;
pub use value_objects::{
    BookingId, Currency, DayStamp, NonEmptyString, WindowDays, WindowId, WindowSpan,
};

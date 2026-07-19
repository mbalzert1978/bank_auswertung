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
//! Fachlicher Kern: die **Materialisierung** des Fensters. Sie kauft Snapshot-Stabilität (welche
//! Zeilen zum Fenster gehören, friert ein), die Barriere als Daten und Determinismus bei
//! Redelivery. Die **Invarianten sitzen an der Konstruktion** von [`AnalysisWindow`] — **nicht
//! leer** und **eine Währung** sind unkonstruierbar zu verletzen, nicht „geprüft" (ADR-0005).
//!
//! Die Seitengröße [`WindowDays`] kommt als **Wert** herein (io liest die Config); die Domäne liest
//! keine Datei und kennt keine Config. Die Zeitachse ist ein sortierbarer [`DayStamp`] (ADR-0013,
//! Entscheidung C) — kein Kalender-Typ, denn die Domäne bindet nichts.

mod entities;
mod error;
mod handler;
mod models;
mod ports;
mod value_objects;

pub use entities::{AnalysisWindow, WindowedBooking};
pub use error::WindowError;
pub use handler::WindowingHandler;
pub use models::AdvanceWindow;
pub use ports::{
    AnalysisWindowRepository, HistoryCompletePublisher, ObfuscatedWindowSource,
    WindowClosedPublisher,
};
pub use value_objects::{
    BookingId, Currency, DayStamp, NonEmptyString, WindowDays, WindowId, WindowSpan,
};

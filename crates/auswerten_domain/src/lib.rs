//! auswerten-Slice — **Domäne** (blind, ohne Dependencies,
//! [ADR-0007](../../../docs/adr/0007-modularer-monolith-feature-slices.md) /
//! [ADR-0013](../../../docs/adr/0013-zeitfenster-collector-paginierender-analyse-kopf.md)).
//!
//! Der vierte Pipeline-Schritt (`einlesen → obfuskieren → fenstern → auswerten → report`) und **die
//! Analyse selbst** ([ADR-0004](../../../docs/adr/0004-domaene-ist-die-analyse-llm-ist-nur-orakel.md):
//! die Domäne ist die Analyse, das LLM nur ein Orakel).
//!
//! `auswerten` hat **zwei Reichweiten** (ADR-0013, Entscheidung E) und dafür **zwei einzweckige
//! Handler**. Gebaut ist die **erste**, die **pro-Fenster**-Reichweite: der
//! [`WindowEvaluationHandler`] konsumiert `FensterGeschlossen`, **liest** das eingefrorene
//! [`AnalysisWindow`] **per ID** ([`AnalysisWindowSource`]) — er bekommt einen **Zeiger**, keine
//! eingebetteten Daten (uniformes ID-Handoff, ADR-0012) —, rechnet die periodische Kennzahl
//! **innerhalb** des Fensters und **legt** das Ergebnis ab ([`EvaluationRepository`]). Beide
//! Schritte sind fehlbar und terminal ([`EvaluationError`]) — kein Dead-Letter, kein Retry.
//!
//! **Fachlicher Kern heute: der Cashflow je Monat.** Das Fenster ist die **Seitengröße** des Pagers
//! (~400 Tage), **nicht** der Berichtszeitraum (CONTEXT.md, markierte Mehrdeutigkeit „Fenster vs.
//! Zeitraum") — gebucketet wird darum **pro Monat innerhalb** des Fensters, und ein Fenster deckt
//! mehrere Buckets ab. Die Kalender-Sicht dafür rekonstruiert dieser Slice **selbst** aus dem
//! sortierbaren Tag-Stamp ([`Month`]), denn die Domäne bindet keine Kalender-Crate.
//!
//! Zu- und Abfluss stehen **getrennt und je vorzeichenlos** ([`Amount`] + [`Direction`], ADR-0005);
//! die **eine** Währung steht **einmal** am Fenster, nicht an jedem Betrag — `fenstern` hat sie an
//! der Konstruktion des Fensters erzwungen, dieser Slice liest den Beweis.
//!
//! **Der [`Fingerprint`] lebt hier** (ADR-0013, Entscheidung F): die **reine Funktion**
//! `Gegenpartei → Schlüssel einer Buchungs*art*`, ohne Port, Zustand oder I/O. Sie schlüsselt eine
//! Firma über ihre Gläubiger-ID, einen **Zahlungsintermediär** zusätzlich über den Händler-Token
//! des Verwendungszwecks (sonst verklumpte der Durchleiter alle Händler dahinter zu einer Art) und
//! eine Privatperson über ihr **seed-basiertes Pseudonym** — das `obfuskieren` stromaufwärts
//! vergeben hat, weshalb der Seed selbst diesen Slice nie erreicht (ADR-0010). Das
//! Pro-Fenster-Ergebnis trägt die **distinkten** Fingerabdrücke seines Fensters; auf ihnen setzen
//! die drei Nutzer auf, die noch kommen.
//!
//! **Noch nicht hier** (je eigenes Folge-Issue): die zweite, **fenster-übergreifende** Reichweite
//! (Wiederkehr über alle Fenster eines Laufs, angestoßen von `HistorieVollständig`), die
//! Fach-Analyzer (Netting, Gebühren-Klassifikation, Pareto), die gelernten Korrekturen, das
//! `Kategorisierer`-Orakel und die Gegenkonto-Auflösung. Dieser Stand ist das **Gerüst plus eine
//! lauffähige Kennzahl und der Schlüssel** — der Pfad, an dem sie andocken.

mod entities;
mod error;
mod handler;
mod models;
mod ports;
mod value_objects;

pub use entities::{AnalysisWindow, WindowEvaluation};
pub use error::EvaluationError;
pub use handler::WindowEvaluationHandler;
pub use models::EvaluateWindow;
pub use ports::{AnalysisWindowSource, EvaluationRepository};
pub use value_objects::{
    Amount, Currency, DayStamp, Direction, Fingerprint, Month, MonthlyCashflow, NonEmptyString,
    Recipient, WindowId, WindowedBooking,
};

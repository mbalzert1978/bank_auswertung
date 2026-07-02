//! CSV-CAMT-v8-Adapter (Baureihenfolge-Schritt 2): implementiert den
//! [`Umsatzquelle`]-Port (inbound) und mappt jede CSV-Zeile auf eine
//! [`domain::Buchung`] — **obfuskiert beim Mappen**, sodass die Domäne nur
//! obfuskierte Value Objects erhält.
//!
//! # Obfuskierung am Adapter (Datenschutz-Invariante)
//!
//! Rohe strukturelle Identifikatoren (eigene IBAN/Kontonummer, Gegen-IBAN, BIC)
//! bleiben ausschließlich hier im Adapter und werden **nie** in eine `Buchung`
//! oder die abgeleitete `KategorieAnfrage` eingebettet:
//!
//! - `Auftragskonto` (eigene IBAN) → obfuskierte [`domain::KontoId`] `KONTO_x`
//!   (First-Seen-Vergabe, für diesen Slice nur laufstabil innerhalb eines
//!   Programmlaufs; die persistente Zuordnung kommt später). Das Auftragskonto
//!   muss ein in der Eigene-Konten-Config hinterlegtes eigenes Konto sein.
//! - Gegen-IBAN → [`domain::Gegenkonto::Eigen`] (mit Konto-Rolle) oder `Fremd`,
//!   aufgelöst über die Eigene-Konten-Config.
//! - Empfänger-Feld: liegt eine Gläubiger-ID vor → [`domain::Empfänger::Firma`]
//!   (die Gläubiger-ID ist öffentlicher Firmen-Anker, kein Identifikator einer
//!   Person); fehlt sie, wird **fail-safe** zu [`domain::Empfänger::Person`] mit
//!   Pseudonym `PERSON_x` (First-Seen). Der teurere Fehler wäre ein geleakter
//!   Klarname, nicht ein Pseudonym zu viel. Der Empfänger-**Klarname** dient dabei
//!   ausschließlich der Pseudonym-Vergabe und wird danach verworfen — er landet
//!   nie in einer `Buchung` (das aktuelle Domänen-Skelett trägt für eine `Firma`
//!   nur die Gläubiger-ID, nicht den Firmennamen — siehe „Offen" in Issue 0003).
//! - Verwendungszweck-Freitext → [`domain::ObfuskierterText`] (leer → `None`,
//!   kein leerer `ObfuskierterText`).
//!
//! Der Adapter sichert **eine einheitliche Währung** über die gesamte Datei zu —
//! eine abweichende Zeile bricht laut ab, statt still falsch zu summieren.
//!
//! # Erwartetes Format (Sparkassen-CSV-CAMT v8)
//!
//! Semikolon-getrennt (`;`), erste Zeile Kopfzeile, Felder optional in `"`
//! gequotet, deutscher Geldbetrag (`-1.234,56`; Minus = Ausgang), Währung als
//! ISO-Code (`EUR`). Der [`CsvQuelle`]-Port liefert bereits nach UTF-8
//! dekodierten Text — eine etwaige Zeichensatz-Umwandlung (real oft
//! ISO-8859-1) lebt am Rand (Composition Root), nicht in der Domäne.

mod adapter;
mod fehler;
mod feldparser;
mod obfuskierung;
mod port;
mod row;

pub use adapter::CsvCamtV8;
pub use fehler::ImportError;
pub use port::{CsvQuelle, CsvQuelleError, Umsatzquelle};

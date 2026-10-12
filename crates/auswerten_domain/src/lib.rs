//! auswerten-Slice — **Domäne** (blind, ohne Dependencies,
//! [ADR-0007](../../../docs/adr/0007-modularer-monolith-feature-slices.md) /
//! [ADR-0013](../../../docs/adr/0013-zeitfenster-collector-paginierender-analyse-kopf.md)).
//!
//! Der vierte Pipeline-Schritt (`einlesen → obfuskieren → fenstern → auswerten → report`) und **die
//! Analyse selbst** ([ADR-0004](../../../docs/adr/0004-domaene-ist-die-analyse-llm-ist-nur-orakel.md):
//! die Domäne ist die Analyse, das LLM nur ein Orakel).
//!
//! `auswerten` hat **zwei Reichweiten** (ADR-0013, Entscheidung E) und dafür **zwei einzweckige
//! Handler**; **beide sind gebaut**. Die **erste** ist die **pro-Fenster**-Reichweite: der
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
//! **Das Gegenkonto ordnet dieser Slice auf Token ein** (ADR-0013, Entscheidung F): das
//! `KONTO_…`-Token einer Buchung wird gegen die [`OwnAccounts`] geprüft — die
//! **token-orientierte** Eigene-Konten-Config, die als **Wert** hereingereicht wird (die Domäne
//! liest keine Datei). Heraus kommt der Summentyp [`CounterAccount`]: `Own(`[`AccountRole`]`)` oder
//! `Foreign` — „Fremd mit Rolle" ist unkonstruierbar, nicht geprüft (ADR-0005). Die **Rolle trägt
//! die Deutung**: ein Zufluss aufs Spar-Konto ist Sparen, nicht Konsum. Eine **rohe IBAN** erreicht
//! diesen Slice dabei zu keinem Zeitpunkt — sie existiert nur vor der Tokenisierung, beim Loader.
//!
//! Dass beide Seiten aus **derselben** Tokenisierung stammen, sichert der [`SeedMarker`] ab: ein
//! opakes Etikett (nicht der Seed), das Fenster und Config je tragen. Stimmen sie nicht überein,
//! endet der Lauf terminal — sonst liefe jede Umbuchung **lautlos** als „fremd" durch und der
//! Bericht wiese sie als Konsum aus, ohne dass irgendetwas fehlschlüge.
//!
//! **Auf dieser Einordnung ruht das Umbuchungs-Netting** (Issue 0004): eine Buchung auf ein eigenes
//! Konto ist kein Abfluss und kein verlorenes Geld (CONTEXT.md, „Umbuchung"), also fällt sie aus dem
//! Cashflow heraus. **Heraus, nicht weg** — sie steht daneben als [`InternalTransfer`], je Monat und
//! Rolle des Gegenkontos: zwei Seiten derselben Sache, deren Summe das Brutto ergibt. Genettet wird
//! **je Zeile, nicht je Paar**: dass das Gegenkonto meines ist, steht schon an der einen Buchung —
//! auf ihr Gegenstück zu warten hieße, jede Umbuchung als Ausgabe zu zählen, deren zweites Bein in
//! einem anderen Fenster, einem anderen Auszug oder gar nicht vorliegt.
//!
//! **Die zweite Reichweite ist die fenster-übergreifende:** der [`HistoryEvaluationHandler`]
//! konsumiert **`HistorieVollständig`** — nicht `FensterGeschlossen`, denn `auswerten` kann die
//! letzte Seite nicht selbst erkennen (der Pager läuft neu→alt; nur `fenstern` kennt die
//! Erschöpfung). Er führt **eine** Query über **alle** Fenster des Laufs ([`OccurrenceSource`]),
//! gescopt über die [`RunSpan`] aus dem Signal, die ihn gegen frühere Läufe abgrenzt, und liefert
//! die [`Occurrence`]s der ganzen Zeitachse zurück. Er **legt nichts ab**: kein Vorkommens-/
//! Fold-Log, keine Akkumulation, keine zweite persistierte Sicht — die Aggregate sind die alleinige
//! Wahrheit. Seine Idempotenz folgt daraus, nicht aus Dedupe-Logik: eine Query über unveränderliche
//! Snapshots antwortet auf erneute Zustellung identisch.
//!
//! **Auf diesen Vorkommen sitzt die Wiederkehr-Erkennung** (Issue 0005): je Kostentreiber
//! ([`CostDriver`]: Buchungsart oder Gebühren eines Kontos) prüft sie die Abflüsse auf einen festen
//! [`Rhythm`] (monatlich, jährlich) bei ähnlichem Betrag und weist eine [`PriceIncrease`] ab einem
//! Stichtag aus. Heraus kommt je Treiber ein [`Recurrence`] mit dem
//! **Jahresbetrag** als Bezugsgröße für spätere Spar-Kandidaten. Sie arbeitet auf **genetteten**
//! Abflüssen: ein Dauerauftrag aufs eigene Konto ist eine Umbuchung, kein Kosten-Treiber — dieselbe
//! Token-Regel wie beim Cashflow, und darum prüft auch diese Reichweite den [`SeedMarker`]. Ihr
//! Ergebnis, die [`HistoryEvaluation`], wird **zurückgegeben**, nie abgelegt.
//!
//! **Auf beidem sitzt der Aderlass** (Issue 0007), ebenfalls in dieser Reichweite, weil nur sie den
//! ganzen Lauf sieht: der genettete Cashflow je eigenem Konto und Monat ([`AccountCashflow`]), das
//! **Pareto-Ranking** der Kostentreiber ([`CostDriver`] → [`RankedDriver`], mit [`Share`] und
//! kumuliertem Anteil am genetteten Abfluss) und die **Spar-Kandidaten** ([`SavingsCandidate`]):
//! jeder Treiber, dessen **Jahresbetrag** die als Wert hereingereichte [`SavingsThreshold`] erreicht
//! — mit Takt und Preissteigerung, wenn er wiederkehrt. Kein LLM ist daran beteiligt. Wiederkehr und
//! Ranking stehen auf **einer** Gruppierung der genetteten Abflüsse nach Treiber. Weil diese
//! Reichweite über alle Fenster summiert, verlangt sie **eine** Währung für den ganzen Lauf
//! ([`RunHistory`], sonst terminal `MixedCurrencies`).
//!
//! **Noch nicht hier** (je eigenes Folge-Issue): die gelernten Korrekturen und ein echtes
//! `Kategorisierer`-Orakel.

mod entities;
mod error;
mod handler;
mod history_handler;
mod models;
mod ports;
mod value_objects;

pub use entities::{AnalysisWindow, WindowEvaluation};
pub use error::EvaluationError;
pub use handler::WindowEvaluationHandler;
pub use history_handler::HistoryEvaluationHandler;
pub use models::{EvaluateHistory, EvaluateWindow};
pub use ports::{
    AnalysisWindowSource, Categorizer, CategoryRegistry, EvaluationRepository, OccurrenceSource,
};
pub use value_objects::{
    AccountCashflow, AccountRole, AccountToken, Amount, BookingKind, Category, CategoryAnswer,
    CategoryRequest, CostDriver, CounterAccount, Counterparty, CreditorId, Currency, DayStamp,
    Direction, Fee, Fingerprint, HistoryEvaluation, InternalTransfer, KindAnchor, Month,
    MonthlyCashflow, NonEmptyString, Occurrence, OwnAccounts, PriceIncrease, RankedDriver,
    Recipient, Recurrence, Rhythm, RunHistory, RunSpan, SavingsCandidate, SavingsThreshold,
    SeedMarker, Share, WindowId, WindowedBooking, is_valid_creditor_id,
};

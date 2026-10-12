//! Die realen io-Lese-Seams des `auswerten`-Slice
//! ([Issue 0058](../../../../docs/issues/0058-auswerten-store-real-und-end-to-end.md)) — die
//! **letzten** Lese-Quellen, die noch als Fake gebunden waren. Sie treten an die Stelle des
//! `FakeBestand` der Fixtures; zusammen mit dem realen Ergebnis-Store
//! ([`AuswertenSqliteStore`](crate::auswerten::AuswertenSqliteStore)) läuft damit die **volle
//! Pipeline** `einlesen → obfuskieren → fenstern → auswerten` end-to-end **ohne einen einzigen
//! Fake** (bis zum `report`).
//!
//! **Ein Struct, zwei Seams, dieselbe Naht:** die pro-Fenster-Reichweite ([`AnalysisWindowSeam`],
//! liest *ein* Fenster per ID) und die fenster-übergreifende ([`OccurrenceSeam`], liest *alle*
//! Fenster einer Lauf-Spanne) setzen ihr Ergebnis beide aus **derselben** Join-Linie zusammen:
//! `fenstern`s Fenster-Tabellen (`auswertungsfenster`/`auswertungsfenster_buchungen`,
//! [Issue 0057](../../../../docs/issues/0057-fenstern-store-real-und-end-to-end.md)) auf die
//! `obfuskiert`-Ablage ([Issue 0056](../../../../docs/issues/0056-obfuskierung-store-real-und-end-to-end.md)).
//! Der Join lebt hier (Beschaffungs-Mechanik), nicht im Handler — die Domäne verlangt *ein* Aggregat
//! bzw. *die Buchungen des Laufs*, nicht die Zerlegung der Ablage in zwei Ports.
//!
//! **Außerhalb der committenden Transaktion** (ADR-0012, uniformes ID-Handoff): beide Tabellen sind
//! append-only, ihr Lesen also message-unabhängig und idempotent — es läuft über
//! [`SqliteOutbox::with_connection`], eine reine Lese-Leihgabe.
//!
//! **Tag-Stamp ↔ ISO-Text:** die Domäne rechnet auf einem sortierbaren `i64`-Tag-Stamp (Tage seit
//! der Unix-Epoche — die Achse, die `auswerten`s Kalender-Rekonstruktion definiert), `obfuskiert`
//! trägt `booking_day` als `YYYY-MM-DD`-Text. Die Übersetzung besorgt der **geteilte**
//! [`calendar`](crate::common::calendar)-Helfer — dieselbe eine Quelle, die auch der `fenstern`-Seam
//! ([`ObfuskiertWindowSource`](crate::fenstern::ObfuskiertWindowSource)) benutzt.
//!
//! **Seed-Marker statt Seed (ADR-0010):** das Fenster braucht das **opake Kennzeichen der
//! Tokenisierung**, unter der seine Token entstanden — nicht den Seed selbst, der diesen Slice nie
//! erreicht. `obfuskiert` legt genau diesen Marker je Zeile ab (abgeleitet **bevor** geschrieben wird,
//! siehe `obfuskierung`s `seed_marker`), sodass dieser Seam ihn nur noch **liest** und gegen die
//! Eigene-Konten-Config hält. Der rohe Seed erreicht damit weder diesen Slice noch die Platte.
//!
//! **Fehlbar → terminal, redigiert:** ein `Err` ist stromabwärts terminal ([`SourceUnavailable`],
//! io-Diagnose); der Text beschreibt ausschließlich die Beschaffungs-Mechanik — dieser Seam liest
//! nur stromabwärts der Datenschutz-Grenze (opake IDs, eine Währung, Token, Tag-Stamps), nie einen
//! personenbezogenen Rohwert.

use auswerten_adapter::{
    AnalysisWindowRow, AnalysisWindowSeam, BookingDirection, BookingRecipient, OccurrenceSeam,
    RunHistoryRow, SourceUnavailable, WindowBookingRow,
};
use rusqlite::{OptionalExtension, params};

use crate::common::calendar;
use crate::obfuskierung::SeedMarker;
use crate::sqlite::SqliteOutbox;

/// Der reale Lese-Seam auf `fenstern`s Fenster-Tabellen, im Join auf die `obfuskiert`-Ablage. `Clone`
/// teilt dasselbe Rückgrat (`Rc` im Innern von [`SqliteOutbox`]).
#[derive(Clone)]
pub struct AuswertungsfensterSource {
    outbox: SqliteOutbox,
}

impl AuswertungsfensterSource {
    /// Bindet den Seam an das gegebene SQLite-Rückgrat (auf dem `fenstern` die Fenster und
    /// `obfuskierung` die Zeilen schreiben). Legt **nichts** an — beide Tabellen gehören ihren
    /// Stores; dieser Seam liest nur.
    pub fn new(outbox: SqliteOutbox) -> Self {
        Self { outbox }
    }

    /// Liest die Buchungen zu einem vorbereiteten Statement — die eine Join-Linie beider Seams. Der
    /// Aufrufer reicht das `sql` und seine Parameter, die Zeilen kommen als **rohe** [`RawBooking`]
    /// zurück (Deutung „außen → innen" folgt außerhalb der Lese-Leihgabe, weil sie fehlbar ist).
    fn raw_bookings(
        &self,
        sql: &str,
        parameters: &[&dyn rusqlite::ToSql],
    ) -> Result<Vec<RawBooking>, SourceUnavailable> {
        self.outbox
            .with_connection(|conn| {
                let mut statement = conn.prepare(sql)?;
                let rows = statement.query_map(parameters, RawBooking::from_row)?;
                rows.collect()
            })
            .map_err(unavailable)
    }
}

impl AnalysisWindowSeam for AuswertungsfensterSource {
    /// Liest das Auswertungsfenster zur ID: erst die Fenster-Zeile (Währung + Existenz), dann ihre
    /// Buchungen im Join auf `obfuskiert`. `Ok(None)` = zu dieser ID liegt **kein** Fenster;
    /// `Err` = die Beschaffung misslang (terminal). Der **Seed-Marker** steht fertig in den
    /// gelesenen Zeilen — sie teilen ihn (eine Tokenisierung je Lauf); ein Fenster ohne Zeilen trägt
    /// keinen, was die ACL beim Deuten terminal als fehlendes Pflichtfeld failt.
    fn read(&self, window_id: &str) -> Result<Option<AnalysisWindowRow>, SourceUnavailable> {
        let currency: Option<String> = self
            .outbox
            .with_connection(|conn| {
                conn.query_row(SELECT_WINDOW, params![window_id], |row| row.get(0))
                    .optional()
            })
            .map_err(unavailable)?;
        let Some(currency) = currency else {
            return Ok(None);
        };
        let raw = self.raw_bookings(SELECT_WINDOW_BOOKINGS, &[&window_id])?;
        let seed_marker = window_seed_marker(&raw)?;
        let bookings = raw
            .iter()
            .map(RawBooking::to_booking)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Some(AnalysisWindowRow {
            currency,
            // Die **eine** Stelle, an der „kein Marker" seine Zeichenketten-Form annimmt: das DTO führt
            // ihn als flache Zelle, und die ACL deutet die leere Zelle terminal als fehlendes
            // Pflichtfeld. Der Übergang steht hier ausgeschrieben, statt als stille Abmachung im
            // Rückgabewert von [`window_seed_marker`] zu stecken.
            seed_marker: seed_marker.map_or_else(String::new, SeedMarker::into_string),
            bookings,
        }))
    }
}

impl OccurrenceSeam for AuswertungsfensterSource {
    /// Liest die Historie **aller** Fenster **innerhalb** der Lauf-Spanne `[from … to]` (beide
    /// Grenzen einschließlich) in **einer** Query über die Fenster-Tabellen im Join auf `obfuskiert`.
    /// Eine Historie **ohne** Buchungen ist kein Fehler, sondern die gültige Auskunft „in dieser
    /// Spanne liegt nichts"; `Err` = die Beschaffung misslang (terminal).
    ///
    /// **Mit Seed-Marker** (Issue 0005): die Wiederkehr-Erkennung nettet Umbuchungen auf Token heraus
    /// und muss dafür wissen, unter welcher Tokenisierung die Gegenkonto-Token entstanden. Gelesen
    /// wird er wie beim Einzel-Fenster — über dieselbe Funktion, die auch hier erzwingt, dass **alle**
    /// Buchungen denselben tragen.
    fn history_in(&self, from: i64, to: i64) -> Result<RunHistoryRow, SourceUnavailable> {
        let raw = self.raw_bookings(SELECT_RUN_BOOKINGS, &[&from, &to])?;
        let seed_marker = window_seed_marker(&raw)?;
        Ok(RunHistoryRow {
            // Wie beim Einzel-Fenster: „kein Marker" wird erst hier zur leeren Zelle — und die ACL
            // verlangt ihn nur, wenn auch Buchungen da sind.
            seed_marker: seed_marker.map_or_else(String::new, SeedMarker::into_string),
            bookings: raw
                .iter()
                .map(RawBooking::to_booking)
                .collect::<Result<Vec<_>, _>>()?,
        })
    }
}

/// Eine **rohe** Buchungs-Zeile, wie der Join sie liefert — die Zellen der `obfuskiert`-Ablage in
/// ihrer Speicher-Form (Text-Tag-Stamp, `i64`-Betrag, Seed-Marker), vor der Deutung „außen → innen". Sie
/// existiert, weil die Deutung fehlbar ist (Tag-Stamp/Richtung) und darum **außerhalb** der
/// rusqlite-Lese-Closure laufen muss.
struct RawBooking {
    booking_day: String,
    amount_minor_units: i64,
    direction: String,
    recipient_kind: String,
    recipient_name: Option<String>,
    recipient_creditor_id: Option<String>,
    recipient_pseudonym: Option<String>,
    purpose: Option<String>,
    counter_account: Option<String>,
    seed_marker: SeedMarker,
    ordering_account: String,
    booking_text: Option<String>,
}

impl RawBooking {
    /// Liest die zwölf vom [`RawBooking`] verwerteten Spalten aus der Join-Zeile (die Reihenfolge
    /// spiegelt die `SELECT`-Liste der beiden Statements).
    fn from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            booking_day: row.get(0)?,
            amount_minor_units: row.get(1)?,
            direction: row.get(2)?,
            recipient_kind: row.get(3)?,
            recipient_name: row.get(4)?,
            recipient_creditor_id: row.get(5)?,
            recipient_pseudonym: row.get(6)?,
            purpose: row.get(7)?,
            counter_account: row.get(8)?,
            seed_marker: SeedMarker::from_stored(row.get(9)?),
            ordering_account: row.get(10)?,
            booking_text: row.get(11)?,
        })
    }

    /// Der opake Seed-Marker dieser Zeile — **gelesen, nicht abgeleitet**: `obfuskierung` legt ihn
    /// beim Ablegen fertig ab, damit der Seed selbst die Platte nie berührt.
    fn seed_marker(&self) -> SeedMarker {
        self.seed_marker.clone()
    }

    /// Deutet die rohe Zeile in die Adapter-Sicht [`WindowBookingRow`] („außen → innen"). Fehlbar
    /// allein über die Speicher-Mechanik: ein unlesbarer Tag-Stamp oder eine Richtung außerhalb des
    /// `CHECK`-Vokabulars failt terminal + redigiert (nie ein Rohwert). Der **vorzeichenlose** Betrag
    /// entsteht aus dem gespeicherten `i64` per `cast_unsigned` — er ist beim Ablegen als Magnitude
    /// `≥ 0` verbürgt (`obfuskierung`s `i64::try_from`), die Umdeutung darum verlustfrei.
    fn to_booking(&self) -> Result<WindowBookingRow, SourceUnavailable> {
        Ok(WindowBookingRow {
            booking_day: day_stamp(&self.booking_day)?,
            amount_minor_units: self.amount_minor_units.cast_unsigned(),
            direction: self.direction()?,
            recipient: self.recipient()?,
            purpose: self.purpose.clone(),
            counter_account: self.counter_account.clone(),
            ordering_account: self.ordering_account.clone(),
            booking_text: self.booking_text.clone(),
        })
    }

    /// Deutet die Richtungs-Zelle. Das `obfuskiert`-`CHECK` lässt nur `incoming`/`outgoing` zu — ein
    /// anderer Wert wäre ein korrupter Bestand und failt terminal (Mechanik, kein Rohwert).
    fn direction(&self) -> Result<BookingDirection, SourceUnavailable> {
        match self.direction.as_str() {
            "incoming" => Ok(BookingDirection::Incoming),
            "outgoing" => Ok(BookingDirection::Outgoing),
            _ => Err(unavailable_message(
                "gespeicherte Richtung außerhalb des Vokabulars",
            )),
        }
    }

    /// Setzt die vier flachen Empfänger-Spalten zum Summentyp [`BookingRecipient`] zusammen — das
    /// Gegenstück zu `obfuskierung`s `RecipientRow`. Die Präsenz der Schlüssel-Zellen (Gläubiger-ID /
    /// Pseudonym) prüft **nicht** hier, sondern die ACL beim Deuten in die Domäne; fehlt eine, reicht
    /// der leere String weiter und wird dort terminal. Eine `recipient_kind` außerhalb des
    /// `CHECK`-Vokabulars ist dagegen ein korrupter Bestand und failt hier terminal.
    fn recipient(&self) -> Result<BookingRecipient, SourceUnavailable> {
        match self.recipient_kind.as_str() {
            "company" => Ok(BookingRecipient::Company {
                name: self.recipient_name.clone(),
                creditor_id: self.recipient_creditor_id.clone().unwrap_or_default(),
            }),
            "person" => Ok(BookingRecipient::Person {
                pseudonym: self.recipient_pseudonym.clone().unwrap_or_default(),
            }),
            "absent" => Ok(BookingRecipient::Absent),
            _ => Err(unavailable_message(
                "gespeicherte Empfänger-Art außerhalb des Vokabulars",
            )),
        }
    }
}

/// Leitet den **einen** Seed-Marker des Fensters — bzw. der Lauf-Historie — aus seinen Zeilen ab
/// und **erzwingt dabei die Invariante**: alle Buchungen eines Fensters (und eines Laufs) stammen aus
/// **einer** Tokenisierung, tragen also denselben Marker. Weichen sie ab (korrupter Bestand oder ein Upstream-Bug), failt das
/// **terminal + redigiert** statt still einen Marker der ersten Zeile zu wählen — sonst könnte die
/// `SeedMarker`-Schranke der Domäne stillschweigend verfälscht werden.
///
/// Ein Fenster **ohne** Zeilen trägt **keinen** Marker — und das sagt der Rückgabetyp: `Ok(None)`,
/// nicht ein leerer [`SeedMarker`]. Ein „leer heißt: gibt es nicht" wäre eine Abmachung, die erst drei
/// Ebenen weiter in der ACL wieder eingelöst wird; wer hier liest, müsste sie kennen, und wer sie dort
/// änderte, bräche diese Stelle lautlos. `Option` sagt dasselbe, ohne dass jemand es wissen muss —
/// abwesend ist abwesend, nicht ein besonderer Wert.
fn window_seed_marker(raw: &[RawBooking]) -> Result<Option<SeedMarker>, SourceUnavailable> {
    let Some(first) = raw.first() else {
        return Ok(None);
    };
    if raw
        .iter()
        .any(|booking| booking.seed_marker != first.seed_marker)
    {
        return Err(unavailable_message(
            "Buchungen aus unterschiedlichen Tokenisierungen",
        ));
    }
    Ok(Some(first.seed_marker()))
}

/// Deutet einen gespeicherten `YYYY-MM-DD`-Textstamp zum sortierbaren `i64`-Tag-Stamp über den
/// geteilten [`calendar`]-Helfer (Tage seit der Unix-Epoche — die Achse, die die Domäne definiert).
/// `Err`, wenn der Bestand einen unlesbaren Textstamp trägt.
fn day_stamp(iso: &str) -> Result<i64, SourceUnavailable> {
    calendar::day_stamp(iso)
        .ok_or_else(|| unavailable_message("gespeicherter Buchungstag unlesbar"))
}

fn unavailable_message(message: &str) -> SourceUnavailable {
    SourceUnavailable {
        message: message.to_string(),
    }
}

fn unavailable(err: rusqlite::Error) -> SourceUnavailable {
    SourceUnavailable {
        message: format!("Auswertungsfenster-Bestand nicht lesbar: {err}"),
    }
}

/// Die Währung des Fensters zur ID — zugleich der Existenz-Test (`optional()` unterscheidet „kein
/// Fenster" von „Beschaffung misslang").
const SELECT_WINDOW: &str = "SELECT currency FROM auswertungsfenster WHERE id = ?1;";

/// Die zehn qualifizierten `obfuskiert`-Spalten, die **beide** Buchungs-Statements lesen — als **ein**
/// Fragment, damit die Liste an einer Stelle mit `RawBooking::from_row` synchron bleibt (nie `SELECT *`,
/// `.rules/sql/queries-und-performance.md`). Ein `macro_rules!` statt `const`, weil `concat!` — das die
/// Fragmente zu den fertigen Statements zusammensetzt — nur String-Literale annimmt.
macro_rules! booking_columns {
    () => {
        "o.booking_day, o.amount_minor_units, o.direction, \
         o.recipient_kind, o.recipient_name, o.recipient_creditor_id, o.recipient_pseudonym, \
         o.purpose, o.counter_account, o.seed_marker, \
         o.ordering_account, o.booking_text"
    };
}

/// Die Buchungen **eines** Fensters, positionsgetreu (die Reihenfolge, in der `fenstern` sie ablegte).
/// Die Spalten-Liste stammt aus `booking_columns!` und deckt sich mit `RawBooking::from_row`.
const SELECT_WINDOW_BOOKINGS: &str = concat!(
    "SELECT ",
    booking_columns!(),
    " FROM auswertungsfenster_buchungen b \
      JOIN obfuskiert o ON o.id = b.booking_id \
      WHERE b.window_id = ?1 \
      ORDER BY b.position;"
);

/// Die Buchungen **aller** Fenster einer Lauf-Spanne: über die Fenster gescopt (`from_day`/`to_day`
/// innerhalb `[?1 … ?2]`), im Join auf `obfuskiert`. Dieselbe Spalten-Liste (`booking_columns!`) wie
/// [`SELECT_WINDOW_BOOKINGS`]; die Ordnung (Tag, dann Fenster, dann Position) ist deterministisch, die
/// fenster-übergreifende Domäne sortiert ohnehin total nach.
const SELECT_RUN_BOOKINGS: &str = concat!(
    "SELECT ",
    booking_columns!(),
    " FROM auswertungsfenster w \
      JOIN auswertungsfenster_buchungen b ON b.window_id = w.id \
      JOIN obfuskiert o ON o.id = b.booking_id \
      WHERE w.from_day >= ?1 AND w.to_day <= ?2 \
      ORDER BY o.booking_day, b.window_id, b.position;"
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fenstern::FensternSqliteStore;
    use crate::obfuskierung::{ObfuskierungSqliteStore, seed_marker};

    /// Der reale Marker ist deterministisch und **nicht** der Seed: gleicher Seed → gleicher Marker,
    /// aber der Seed selbst steckt nicht drin (keyed Hash über ein festes Etikett).
    #[test]
    fn der_seed_marker_ist_deterministisch_und_verbirgt_den_seed() {
        let seed = 42u64;
        assert_eq!(seed_marker(seed), seed_marker(seed));
        assert_ne!(seed_marker(seed), seed_marker(seed + 1));
        assert!(
            !seed_marker(seed).as_str().contains(&seed.to_string()),
            "der Marker darf den rohen Seed nicht enthalten"
        );
    }

    /// Eine fehlende `auswertungsfenster`-Tabelle failt die pro-Fenster-Lese terminal + redigiert
    /// (Beschaffungs-Mechanik, nie ein Rohwert).
    #[expect(
        clippy::panic_in_result_fn,
        reason = "Test-Helfer: die assert! ist die Spezifikation dieses Verhaltens"
    )]
    #[test]
    fn read_ohne_fenster_tabelle_failt_terminal() -> rusqlite::Result<()> {
        let outbox = SqliteOutbox::open_in_memory()?;
        // Bewusst **keine** Fenster-Tabellen angelegt.
        let source = AuswertungsfensterSource::new(outbox);

        let fehler = source
            .read("egal")
            .expect_err("ohne Fenster-Tabelle muss die Lese terminal failen");
        assert!(
            !format!("{fehler}").is_empty(),
            "der terminale Fehler trägt eine (redigierte) Diagnose"
        );
        Ok(())
    }

    /// Ebenso terminal failt die fenster-übergreifende Query ohne die Fenster-Tabellen.
    #[test]
    fn history_in_ohne_fenster_tabelle_failt_terminal() -> rusqlite::Result<()> {
        let outbox = SqliteOutbox::open_in_memory()?;
        let source = AuswertungsfensterSource::new(outbox);

        source
            .history_in(0, 1_000_000)
            .expect_err("ohne Fenster-Tabellen muss die Query terminal failen");
        Ok(())
    }

    /// Legt **zwei Fenster** eines Laufs real an — je eines mit **einer** obfuskierten Buchung,
    /// deren Seed-Marker aus dem gegebenen Seed stammt. Juni und Juli 2024, Tag-Stamps 19 884 und
    /// 19 914.
    fn zwei_fenster_mit_seeds(
        outbox: &SqliteOutbox,
        seed_juni: u64,
        seed_juli: u64,
    ) -> rusqlite::Result<()> {
        ObfuskierungSqliteStore::with_own_tables(outbox.clone())?;
        FensternSqliteStore::with_own_tables(outbox.clone())?;
        outbox.with_connection(|conn| {
            for (id, day, stamp, seed) in [
                ("juni", "2024-06-10", 19_884_i64, seed_juni),
                ("juli", "2024-07-10", 19_914_i64, seed_juli),
            ] {
                conn.execute(
                    "INSERT INTO obfuskiert (id, source_id, ordering_account, amount_minor_units, \
                     currency, direction, recipient_kind, booking_day, value_date, seed_marker) \
                     VALUES (?1, ?1, 'KONTO_1', 1299, 'EUR', 'outgoing', 'absent', ?2, ?2, ?3);",
                    params![id, day, seed_marker(seed).as_str()],
                )?;
                conn.execute(
                    "INSERT INTO auswertungsfenster (id, from_day, to_day, currency) \
                     VALUES (?1, ?2, ?2, 'EUR');",
                    params![id, stamp],
                )?;
                conn.execute(
                    "INSERT INTO auswertungsfenster_buchungen (window_id, position, booking_id) \
                     VALUES (?1, 0, ?1);",
                    params![id],
                )?;
            }
            Ok(())
        })
    }

    /// Die Lauf-Historie über **zwei** Fenster aus **verschiedenen** Tokenisierungen ist ein
    /// korrupter Bestand: die Query failt terminal + redigiert, statt still den Marker der ersten
    /// Zeile zu nehmen — sonst würde die Seed-Marker-Prüfung der Wiederkehr-Erkennung (Issue 0005)
    /// gegen einen Wert geführt, der nur für einen Teil der Buchungen gilt.
    #[expect(
        clippy::panic_in_result_fn,
        reason = "Test-Helfer: die assert! ist die Spezifikation dieses Verhaltens"
    )]
    #[test]
    fn history_in_ueber_fenster_mit_verschiedenen_seeds_failt_terminal() -> rusqlite::Result<()> {
        let outbox = SqliteOutbox::open_in_memory()?;
        zwei_fenster_mit_seeds(&outbox, 42, 43)?;
        let source = AuswertungsfensterSource::new(outbox);

        let fehler = source
            .history_in(19_884, 19_914)
            .expect_err("gemischte Tokenisierungen im Lauf müssen terminal failen");
        assert!(
            !fehler.message.contains(seed_marker(42).as_str())
                && !fehler.message.contains(seed_marker(43).as_str()),
            "die Diagnose nennt keinen der Marker"
        );
        Ok(())
    }

    /// Die Gegenprobe: tragen beide Fenster **denselben** Marker, liefert die Query beide Buchungen
    /// samt diesem einen Marker.
    #[expect(
        clippy::panic_in_result_fn,
        reason = "Test-Helfer: die assert! ist die Spezifikation dieses Verhaltens"
    )]
    #[test]
    fn history_in_ueber_fenster_einer_tokenisierung_traegt_deren_marker() -> rusqlite::Result<()> {
        let outbox = SqliteOutbox::open_in_memory()?;
        zwei_fenster_mit_seeds(&outbox, 42, 42)?;
        let source = AuswertungsfensterSource::new(outbox);

        let historie = source.history_in(19_884, 19_914);
        assert_eq!(
            historie
                .as_ref()
                .map(|h| (h.seed_marker.clone(), h.bookings.len())),
            Ok((seed_marker(42).as_str().to_string(), 2))
        );
        Ok(())
    }

    /// Ein Zeiger, der ins Leere zeigt (Fenster-Tabelle da, aber ohne diese ID), ist **kein**
    /// Beschaffungs-Fehler, sondern die gültige Auskunft `Ok(None)` — die der Slice dann terminal als
    /// „Fenster nicht gefunden" deutet.
    #[expect(
        clippy::panic_in_result_fn,
        reason = "Test-Helfer: die assert! ist die Spezifikation dieses Verhaltens"
    )]
    #[test]
    fn read_eines_unbekannten_fensters_ist_ok_none() -> rusqlite::Result<()> {
        let outbox = SqliteOutbox::open_in_memory()?;
        // Legt die (leere) auswertungsfenster-Tabelle an.
        FensternSqliteStore::with_own_tables(outbox.clone())?;
        let source = AuswertungsfensterSource::new(outbox);

        assert!(
            matches!(source.read("gibt-es-nicht"), Ok(None)),
            "ein unbekanntes Fenster ist Ok(None), kein Fehler"
        );
        Ok(())
    }

    /// Baut eine rohe Zeile mit einem gegebenen `seed`; die übrigen Zellen sind für die
    /// Marker-Ableitung belanglos.
    fn raw_with_seed(seed: u64) -> RawBooking {
        RawBooking {
            booking_day: "2024-06-20".to_string(),
            amount_minor_units: 1000,
            direction: "outgoing".to_string(),
            recipient_kind: "absent".to_string(),
            recipient_name: None,
            recipient_creditor_id: None,
            recipient_pseudonym: None,
            purpose: None,
            counter_account: None,
            seed_marker: seed_marker(seed),
            ordering_account: "KONTO_1".to_string(),
            booking_text: None,
        }
    }

    /// Tragen alle Zeilen denselben Seed, ist der Marker der ihre — eine Tokenisierung je Fenster.
    #[test]
    fn ein_einheitlicher_seed_ergibt_den_marker_dieser_tokenisierung() {
        let raw = vec![raw_with_seed(42), raw_with_seed(42)];
        assert_eq!(window_seed_marker(&raw), Ok(Some(seed_marker(42))));
    }

    /// Ein Fenster **ohne** Zeilen trägt keinen Marker — und sagt das als `Ok(None)`, nicht als
    /// leerer Wert, den erst die ACL wieder als „fehlt" deuten müsste.
    #[test]
    fn ein_fenster_ohne_zeilen_traegt_keinen_marker() {
        assert_eq!(window_seed_marker(&[]), Ok(None));
    }

    /// Gemischte Seeds sind ein korrupter Bestand: fail-fast terminal + redigiert, nie still der
    /// Marker der ersten Zeile.
    #[test]
    fn gemischte_seeds_failen_terminal_statt_still_den_ersten_zu_nehmen() {
        let raw = vec![raw_with_seed(42), raw_with_seed(43)];
        assert!(
            window_seed_marker(&raw).is_err(),
            "unterschiedliche Tokenisierungen im selben Fenster müssen terminal failen"
        );
    }
}

//! Gemeinsames Test-Gerüst für die slice-spezifischen Store-Tests: die reale Vorstufen-Pipeline
//! **einmal** verdrahtet, statt sie in jedem Store-Testmodul von Hand nachzubauen.
//!
//! Die drei realen Stores (`einlesen`/`obfuskierung`/`fenstern`) prüfen ihr Verhalten end-to-end
//! über der echten Kette (CSV-Quelle → reale Seams → reale Tabellen → echter Broker). Jeder neue
//! Store braucht dabei denselben Bestand seiner Vorstufen — dieses Modul stellt ihn **an einer
//! Stelle** bereit: den CSV-Reader auf die Fixtures ([`csv_source`]) und das Seeding des realen
//! Bestands bis zu einer gewählten [`Stage`] ([`seed_through`]). Das ganze Modul ist
//! testgebunden (`#[cfg(test)]` an der Modul-Deklaration in [`lib`](crate)); seine Items sind
//! crate-privat.

use std::cell::RefCell;
use std::rc::Rc;

use crate::Dispatcher;
use crate::MessageId;
use crate::common::calendar;
use crate::einlesen::{
    CONSUMED_MESSAGE_TYPE as EINLESEN_CONSUMED, CsvCamtV8Source, EinlesenSqliteStore,
    GermanCalendar, GermanMoney, IsoCurrencies, consumer as einlesen_consumer,
};
use crate::obfuskierung::{
    CONSUMED_MESSAGE_TYPE as OBFUSKIERUNG_CONSUMED, EinlesenRowSource, KnownPersonNames,
    ObfuskierungSqliteStore, SeededObfuscator, consumer as obfuskierung_consumer,
};
use crate::outbox::{OutboxError, OutboxHandler, OutboxMessage, UnitOfWork};
use crate::sqlite::{Delivery, SqliteOutbox};

/// Registriert einen frischen Consumer und stellt sicher, dass er **keinen** bestehenden verdrängt —
/// die eine Quelle je Dispatch-Key. Bündelt das über alle Store-Testmodule wiederholte
/// `assert!(dispatcher.register(…).is_none(), "…")`, sodass eine Registrierung wie eine
/// schlichte Zeile liest statt als Glied einer 40-zeiligen `&&`-Konjunktion.
#[track_caller]
pub(crate) fn register_fresh<'a>(
    dispatcher: &mut Dispatcher<'a>,
    message_type: &str,
    consumer: Box<dyn OutboxHandler + 'a>,
) {
    assert!(
        dispatcher.register(message_type, consumer).is_none(),
        "frische Registrierung darf keinen Consumer verdrängen ({message_type})"
    );
}

/// Nagelt einen terminalen, redigierten Fehlschlag **beidseitig** fest: der eine `Warning`-Ausgang —
/// vom Broker sofort ausgesondert (Issue 0085) — trägt das erwartete Diagnose-Präfix des jeweiligen Seams (**positiv**) **und** keinen Rohwert
/// (**negativ**). Bündelt die über die Store-Testmodule wiederholte Zusicherung an **einer** Stelle
/// ([Issue 0079](../../../../docs/issues/0079-leak-tests-auf-das-diagnose-praefix-festnageln.md)):
/// eine nur negative Prüfung bliebe grün, fiele die durchgereichte Diagnose ganz weg oder käme sie
/// aus einer anderen Quelle — genau der Regress, den das Präfix ausschließt.
#[expect(
    clippy::panic,
    reason = "Test-Helfer: genau ein Warning-Ausgang ist die Spezifikation dieses Verhaltens"
)]
#[track_caller]
pub(crate) fn assert_redacted_diagnosis(
    outcomes: &[(OutboxMessage, Delivery)],
    expected_prefix: &str,
    raw_value: &str,
) {
    let reason = match outcomes {
        [
            (
                _,
                Delivery::DeadLettered {
                    error: OutboxError::Warning { reason },
                    ..
                },
            ),
        ] => reason,
        other => panic!("genau ein terminaler Warning-Ausgang erwartet: {other:?}"),
    };
    assert!(
        reason.contains(expected_prefix),
        "die Diagnose muss das Präfix `{expected_prefix}` tragen (sonst ist sie weg oder stammt aus \
         anderer Quelle): {reason}"
    );
    assert!(
        !reason.contains(raw_value),
        "die Diagnose darf keinen Rohwert tragen (Redaktion) — Präfix `{expected_prefix}`"
    );
}

/// Wurde **jede** Nachricht des Aufrufs verarbeitet? Die eine Zusicherung „die Kette lief ohne
/// Fehlschlag durch", statt dasselbe `matches!` in jedem Store-Testmodul.
pub(crate) fn all_processed(deliveries: &[(OutboxMessage, Delivery)]) -> bool {
    deliveries
        .iter()
        .all(|(_, delivery)| matches!(delivery, Delivery::Processed))
}

/// Der feste Test-Seed, mit dem das Seeding den obfuskierten Bestand tokenisiert. Downstream-Stores
/// (`fenstern`) sind blind für seinen Wert — sie brauchen nur einen **konsistent** getokenisierten
/// Bestand; `obfuskierung`s eigene Tests importieren ihn als den Seed, dessen Wirkung sie prüfen.
pub(crate) const SEED: u64 = 42;

/// Eine **unerreichbare** Spar-Schwelle (Issue 0007) für Tests, die die Spar-Kandidaten nicht
/// betrachten: kein Treiber erreicht sie, die Kandidaten-Liste bleibt leer.
pub(crate) const NO_SAVINGS: auswerten_adapter::SavingsThresholdConfig =
    auswerten_adapter::SavingsThresholdConfig {
        annual_minor_units: std::num::NonZeroU64::MAX,
    };

/// Baut den echten CSV-CAMT-v8-Reader auf die benannte `einlesen`-Fixture (Pfad relativ zum
/// io-Crate). Ein und dieselbe Fixture-Quelle für alle drei Store-Testmodule.
pub(crate) fn csv_source(name: &str) -> CsvCamtV8Source {
    let path = format!(
        "{}/fixtures/einlesen/csv/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    CsvCamtV8Source::new(path)
}

/// Bis zu welcher Vorstufe [`seed_through`] den realen Bestand aufbaut.
pub(crate) enum Stage {
    /// Nur `einlesen`: die reale(n) `einlesen_lines`-Zeile(n) plus die offenen `Ingested`-Zeiger.
    Einlesen,
    /// `einlesen` **und** `obfuskierung`: zusätzlich der seed-basiert getokenisierte
    /// `obfuskiert`-Bestand plus die offenen `Obfuscated`-Zeiger.
    Obfuskierung,
}

/// Seedet den **realen** Vorstufen-Bestand auf **einem** Rückgrat bis zur gewählten [`Stage`]: erst
/// der `einlesen`-Consumer über der CSV-Fixture (landet die reale(n) `einlesen_lines`-Zeile(n) und
/// publiziert `Ingested`), dann — ab [`Stage::Obfuskierung`] — der `obfuskierung`-Consumer über
/// **allen** dabei publizierten `Ingested`-Nachrichten (landet den getokenisierten Bestand und
/// publiziert `Obfuscated`). Jede Stufe wird über ihren eigenen [`Dispatcher`] zugestellt; die
/// `assert!`-Invarianten (jede Vorstufe verarbeitet atomar) sind Teil der Spezifikation.
#[expect(
    clippy::panic_in_result_fn,
    reason = "Test-Helfer: die assert! prüfen die Vorstufen-Invarianten vor dem eigentlichen Lauf"
)]
pub(crate) fn seed_through(
    outbox: &SqliteOutbox,
    stage: Stage,
    fixture: &str,
) -> rusqlite::Result<()> {
    // Stufe einlesen: realer Store + reale Seams über der Fixture.
    let einlesen_store = EinlesenSqliteStore::with_own_tables(outbox.clone())?;
    let einlesen_source = csv_source(fixture);
    let einlesen = einlesen_consumer(
        &einlesen_source,
        &GermanCalendar,
        &GermanMoney,
        &IsoCurrencies,
        einlesen_store,
    );
    let mut stage1 = Dispatcher::new();
    register_fresh(&mut stage1, EINLESEN_CONSUMED, Box::new(einlesen));
    outbox.enqueue(EINLESEN_CONSUMED, fixture)?;
    let einlesen_out = outbox.deliver_open(&stage1)?;
    assert!(
        all_processed(&einlesen_out),
        "einlesen muss atomar verarbeiten: {einlesen_out:?}"
    );

    if matches!(stage, Stage::Einlesen) {
        return Ok(());
    }

    // Stufe obfuskierung: realer Store, reale Lese-Quelle, realer Obfuskator über allen Ingested.
    let source = EinlesenRowSource::new(outbox.clone());
    let obfuscator = SeededObfuscator;
    let obfuskierung_store = ObfuskierungSqliteStore::with_own_tables(outbox.clone())?;
    // Die reale Namensliste: Empfänger ohne Gläubiger-ID aus dem eben geseedeten einlesen-Bestand.
    let persons = KnownPersonNames::new(outbox.clone(), Vec::new());
    let obfuskierung =
        obfuskierung_consumer(&source, &obfuscator, &persons, SEED, obfuskierung_store);
    let mut stage2 = Dispatcher::new();
    register_fresh(&mut stage2, OBFUSKIERUNG_CONSUMED, Box::new(obfuskierung));
    let obfuskierung_out = outbox.deliver_open(&stage2)?;
    assert!(
        all_processed(&obfuskierung_out),
        "obfuskierung muss atomar verarbeiten: {obfuskierung_out:?}"
    );
    Ok(())
}

/// RAII-Wächter über einen eindeutigen temporären DB-Pfad: löscht die Datei beim Fallenlassen — auch
/// wenn ein `assert!` den Test abbricht. Hier statt in einem einzelnen Testmodul, weil ihn jeder Test
/// braucht, der das **datei-gestützte** Rückgrat prüft ([`schema::open`](crate::schema::open)) statt des flüchtigen
/// In-Memory-Rückgrats: die Resume-Zusage über einen Prozess-Neustart und der reale Lauf
/// ([`run`](crate::run)).
pub(crate) struct TempDbPath(std::path::PathBuf);

impl TempDbPath {
    /// Ein frischer, eindeutiger Pfad im Temp-Verzeichnis (Prozess-ID + Nanosekunden), damit
    /// parallel laufende Tests einander nicht in dieselbe Datei schreiben.
    pub(crate) fn new(tag: &str) -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|dauer| dauer.as_nanos())
            .unwrap_or(0);
        let mut pfad = std::env::temp_dir();
        pfad.push(format!(
            "bank_auswertung_outbox_{tag}_{}_{nanos}.sqlite",
            std::process::id()
        ));
        Self(pfad)
    }

    /// Der Pfad, den der Test der [`AppConfig`](crate::config::AppConfig) bzw. den
    /// [`RunPaths`](crate::run::RunPaths) mitgibt.
    pub(crate) fn path(&self) -> &std::path::Path {
        &self.0
    }
}

impl Drop for TempDbPath {
    fn drop(&mut self) {
        // Best-effort-Aufräumen: eine bereits fehlende Datei ist kein Fehler.
        match std::fs::remove_file(&self.0) {
            Ok(()) | Err(_) => {}
        }
    }
}

/// Der `i64`-Tag-Stamp eines Kalendertags über den geteilten
/// [`calendar`](crate::common::calendar)-Helfer (Tage seit der Unix-Epoche) — dieselbe Achse, die die
/// realen Stores ablegen und die realen Lese-Seams zurückgeben. Eine Quelle für alle Store-Testmodule
/// statt eines je Modul nachgebauten `stamp`.
pub(crate) fn stamp(year: i32, month: u32, day: u32) -> i64 {
    calendar::day_stamp(&format!("{year:04}-{month:02}-{day:02}")).expect("gültiges Testdatum")
}

/// Ein Ack-Handler für die Store-Testmodule: ackt jede zugestellte Nachricht ohne eigene fachliche
/// Wirkung und protokolliert dabei `(message_type, partition_key)` in Zustellreihenfolge. Er deckt
/// zwei Bedürfnisse mit **einem** Typ: Nachrichtentypen, die der jeweilige Test bewusst nicht
/// stromabwärts konsumiert, offen abzuräumen (sonst konvergiert die Broker-Schleife nie), und ihr
/// Auftreten zu belegen. Wer das Protokoll nicht braucht, ignoriert schlicht
/// [`seen`](CapturingAck::seen). `Clone` teilt denselben Puffer (`Rc` im Innern), sodass **eine**
/// Instanz für mehrere Dispatch-Keys registriert werden kann und alle in denselben Puffer schreiben.
#[derive(Clone)]
pub(crate) struct CapturingAck {
    outbox: SqliteOutbox,
    seen: Rc<RefCell<Vec<(String, String)>>>,
}

impl CapturingAck {
    /// Bindet den Handler an das Rückgrat, auf dem er ackt (eine geteilte `Rc`-Leihgabe von
    /// [`SqliteOutbox`]), mit leerem Protokoll.
    pub(crate) fn new(outbox: SqliteOutbox) -> Self {
        Self {
            outbox,
            seen: Rc::new(RefCell::new(Vec::new())),
        }
    }

    /// Die protokollierten `(message_type, partition_key)` in Zustellreihenfolge — eine Kopie, damit
    /// der Aufrufer sie frei filtern kann, ohne die Borrow zu halten.
    pub(crate) fn seen(&self) -> Vec<(String, String)> {
        self.seen.borrow().clone()
    }
}

impl OutboxHandler for CapturingAck {
    fn handle(&self, message: OutboxMessage) -> Result<(), OutboxError> {
        self.seen
            .borrow_mut()
            .push((message.message_type.clone(), message.partition_key.clone()));
        self.outbox
            .begin(MessageId(&message.id))
            .commit()
            .map_err(|err| OutboxError::Retry {
                reason: err.to_string(),
            })
    }
}

/// Treibt die Outbox, bis keine offenen Nachrichten mehr übrig sind — nötig überall dort, wo ein
/// Consumer sich selbst fortsetzt (`fenstern` reiht je Seite die nächste ein). Bricht wie ein realer
/// Broker beim ersten Fehler in einer Runde ab (keine halb-fortgesetzte Kette), statt ihn zu
/// wiederholen. Gedeckelt: ein nicht terminierender Lauf muss den Test laut scheitern lassen, statt
/// ihn hängen zu lassen.
#[expect(
    clippy::panic_in_result_fn,
    clippy::panic,
    reason = "Test-Helfer: ein nicht terminierender Lauf ist ein Regress und muss hart scheitern"
)]
pub(crate) fn drain(
    outbox: &SqliteOutbox,
    dispatcher: &Dispatcher<'_>,
) -> rusqlite::Result<Vec<(OutboxMessage, Delivery)>> {
    let mut all = Vec::new();
    for _ in 0..32 {
        let batch = outbox.deliver_open(dispatcher)?;
        if batch.is_empty() {
            return Ok(all);
        }
        let failed = !all_processed(&batch);
        all.extend(batch);
        if failed {
            return Ok(all);
        }
    }
    panic!("Broker-Schleife terminierte nicht (32 Runden)");
}

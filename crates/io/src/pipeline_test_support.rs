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

use crate::Dispatcher;
use crate::einlesen::{
    CONSUMED_MESSAGE_TYPE as EINLESEN_CONSUMED, CsvCamtV8Source, EinlesenSqliteStore,
    GermanCalendar, GermanMoney, IsoCurrencies, consumer as einlesen_consumer,
};
use crate::obfuskierung::{
    CONSUMED_MESSAGE_TYPE as OBFUSKIERUNG_CONSUMED, EinlesenRowSource, ObfuskierungSqliteStore,
    SeededObfuscator, consumer as obfuskierung_consumer,
};
use crate::outbox::OutboxHandler;
use crate::sqlite::SqliteOutbox;

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

/// Der feste Test-Seed, mit dem das Seeding den obfuskierten Bestand tokenisiert. Downstream-Stores
/// (`fenstern`) sind blind für seinen Wert — sie brauchen nur einen **konsistent** getokenisierten
/// Bestand; `obfuskierung`s eigene Tests importieren ihn als den Seed, dessen Wirkung sie prüfen.
pub(crate) const SEED: u64 = 42;

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
    let einlesen_store = EinlesenSqliteStore::new(outbox.clone())?;
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
        einlesen_out.iter().all(|(_, outcome)| outcome.is_ok()),
        "einlesen muss atomar verarbeiten: {einlesen_out:?}"
    );

    if matches!(stage, Stage::Einlesen) {
        return Ok(());
    }

    // Stufe obfuskierung: realer Store, reale Lese-Quelle, realer Obfuskator über allen Ingested.
    let source = EinlesenRowSource::new(outbox.clone());
    let obfuscator = SeededObfuscator;
    let obfuskierung_store = ObfuskierungSqliteStore::new(outbox.clone())?;
    let obfuskierung = obfuskierung_consumer(&source, &obfuscator, SEED, obfuskierung_store);
    let mut stage2 = Dispatcher::new();
    register_fresh(&mut stage2, OBFUSKIERUNG_CONSUMED, Box::new(obfuskierung));
    let obfuskierung_out = outbox.deliver_open(&stage2)?;
    assert!(
        obfuskierung_out.iter().all(|(_, outcome)| outcome.is_ok()),
        "obfuskierung muss atomar verarbeiten: {obfuskierung_out:?}"
    );
    Ok(())
}

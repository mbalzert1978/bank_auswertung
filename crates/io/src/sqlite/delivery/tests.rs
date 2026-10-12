//! Versuchszählung und Aussondern gegen **echtes** SQLite (Issue 0085): je Akzeptanzkriterium ein
//! Test, über den realen Broker, den generischen [`OutboxConsumer`] und die echte Unit — nur der
//! Slice-Handler ist ein Stellvertreter, der so scheitert, wie der Test es vorgibt.

use std::cell::Cell;
use std::marker::PhantomData;
use std::rc::Rc;

use application_kernel::CommandDecoder;

use super::*;
use crate::run_in_unit;
use crate::{Dispatcher, MessageId, OutboxConsumer, TransactionError, TransactionalHandler};

const GIFT: &str = "Gift";
const GESUND: &str = "Gesund";

/// Ein Rohwert, wie er in einem `partition_key` stehen **könnte**, und ein Freitext als `content` —
/// beides darf in keinem Aussonderungs-Grund landen.
const ROHE_IBAN: &str = "DE89370400440532013000";
const FREITEXT: &str = "Miete Mustermann Juni";

struct Cmd;

/// Baut aus jedem Inhalt ein Kommando.
struct JederInhalt;
impl CommandDecoder for JederInhalt {
    type Command = Cmd;
    fn decode(_partition_key: &str, _content: &str) -> Option<Cmd> {
        Some(Cmd)
    }
}

/// Baut aus keinem Inhalt ein Kommando — die Hülle trägt nicht (`Critical`).
struct KeinInhalt;
impl CommandDecoder for KeinInhalt {
    type Command = Cmd;
    fn decode(_partition_key: &str, _content: &str) -> Option<Cmd> {
        None
    }
}

/// Der nackte, redigierte Domänen-Fehler des Stellvertreter-Slice: nur ein statischer Feldname.
#[derive(Debug)]
struct PflichtfeldFehlt;
impl std::fmt::Display for PflichtfeldFehlt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Pflichtfeld betrag fehlt")
    }
}
impl std::error::Error for PflichtfeldFehlt {}

/// Wie der Versuch des Stellvertreters ausgeht.
#[derive(Clone, Copy)]
enum Ausgang {
    /// Der Handler gelingt, die Unit committet.
    Gelingt,
    /// Der Handler scheitert **terminal** mit seinem Domänen-Fehler.
    Terminal,
    /// Der Handler gelingt, aber der **Commit** scheitert an SQLite — bis einschließlich zum
    /// angegebenen Versuch; danach gelingt er.
    CommitScheitertBis(u32),
}

/// Der Slice-Stellvertreter hinter dem generischen [`OutboxConsumer`]: zählt seine Versuche und geht
/// aus, wie [`Ausgang`] es vorgibt.
struct Stellvertreter<D> {
    outbox: SqliteOutbox,
    versuche: Rc<Cell<u32>>,
    ausgang: Ausgang,
    decoder: PhantomData<fn() -> D>,
}

impl<D: CommandDecoder<Command = Cmd>> TransactionalHandler for Stellvertreter<D> {
    type Command = Cmd;
    type Decoder = D;
    type Error = PflichtfeldFehlt;

    fn handle(
        &self,
        ack: MessageId<'_>,
        _command: Cmd,
    ) -> Result<(), TransactionError<PflichtfeldFehlt>> {
        self.versuche.set(self.versuche.get() + 1);
        let versuch = self.versuche.get();
        let ausgang = self.ausgang;
        run_in_unit(self.outbox.begin(ack), move |unit, _publishing| {
            match ausgang {
                Ausgang::Terminal => return Err(PflichtfeldFehlt),
                Ausgang::CommitScheitertBis(bis) if versuch <= bis => {
                    // Eine Schreibung in eine fehlende Tabelle: der Commit scheitert an SQLite selbst.
                    unit.stage_row(Box::new(|tx| {
                        tx.execute("INSERT INTO gibt_es_nicht (x) VALUES (1);", ())
                            .map(|_| ())
                    }));
                }
                Ausgang::Gelingt | Ausgang::CommitScheitertBis(_) => {}
            }
            Ok(())
        })
    }
}

/// Registriert für `message_type` einen Stellvertreter mit dem gegebenen Ausgang und gibt seinen
/// Versuchszähler zurück.
fn registriere<D: CommandDecoder<Command = Cmd> + 'static>(
    dispatcher: &mut Dispatcher<'static>,
    outbox: &SqliteOutbox,
    message_type: &str,
    ausgang: Ausgang,
) -> Rc<Cell<u32>> {
    let versuche = Rc::new(Cell::new(0));
    let consumer = OutboxConsumer::new(Stellvertreter::<D> {
        outbox: outbox.clone(),
        versuche: Rc::clone(&versuche),
        ausgang,
        decoder: PhantomData,
    });
    assert!(
        dispatcher
            .register(message_type, Box::new(consumer))
            .is_none(),
        "frische Registrierung"
    );
    versuche
}

/// Status und Versuchszahl einer Nachricht, direkt aus ihrer Outbox-Zeile.
fn zeile(outbox: &SqliteOutbox, id: &str) -> rusqlite::Result<(String, u32)> {
    outbox.with_connection(|conn| {
        conn.query_row(
            "SELECT status, attempt FROM outbox WHERE id = ?1;",
            [id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
    })
}

/// Reiht eine Nachricht mit einem Rohwert als `partition_key` und Freitext als `content` ein.
fn reihe_mit_rohwert_ein(outbox: &SqliteOutbox, message_type: &str) -> rusqlite::Result<String> {
    let id = outbox.enqueue(message_type, ROHE_IBAN)?;
    outbox.with_connection(|conn| {
        conn.execute(
            "UPDATE outbox SET content = ?2 WHERE id = ?1;",
            [id.as_str(), FREITEXT],
        )
    })?;
    Ok(id)
}

#[expect(
    clippy::panic_in_result_fn,
    reason = "Test: die assert! sind die Spezifikation dieses Verhaltens"
)]
#[test]
fn ein_terminal_scheiternder_handler_wird_beim_ersten_fehlschlag_ausgesondert_und_nie_wieder_zugestellt()
-> rusqlite::Result<()> {
    let outbox = SqliteOutbox::open_in_memory()?;
    let mut dispatcher = Dispatcher::new();
    let versuche = registriere::<JederInhalt>(&mut dispatcher, &outbox, GIFT, Ausgang::Terminal);
    let gift = outbox.enqueue(GIFT, "zeiger-1")?;

    let erste_runde = outbox.deliver_open(&dispatcher)?;

    assert!(
        matches!(
            erste_runde.as_slice(),
            [(message, Delivery::DeadLettered(OutboxError::Warning { .. }))] if message.id == gift
        ),
        "terminal ⇒ beim ersten Fehlschlag ausgesondert: {erste_runde:?}"
    );
    assert_eq!(zeile(&outbox, &gift)?, ("dead_letter".to_string(), 1));
    // Ein Folgelauf stellt sie nicht erneut zu.
    assert!(outbox.deliver_open(&dispatcher)?.is_empty());
    assert_eq!(versuche.get(), 1, "genau ein Versuch");
    Ok(())
}

#[expect(
    clippy::panic_in_result_fn,
    reason = "Test: die assert! sind die Spezifikation dieses Verhaltens"
)]
#[test]
fn eine_hoffnungslose_huelle_wird_als_critical_sofort_ausgesondert() -> rusqlite::Result<()> {
    // Zwei Fälle derselben Ebene: kein Consumer für den Dispatch-Key, und ein Inhalt, aus dem der
    // Decoder kein Kommando baut. Beide sind terminal — Wiederholen änderte nichts.
    let outbox = SqliteOutbox::open_in_memory()?;
    let mut dispatcher = Dispatcher::new();
    registriere::<KeinInhalt>(&mut dispatcher, &outbox, GIFT, Ausgang::Gelingt);
    let unlesbar = outbox.enqueue(GIFT, "zeiger-1")?;
    let unbekannt = outbox.enqueue("NiemandHoertZu", "zeiger-2")?;

    let runde = outbox.deliver_open(&dispatcher)?;

    assert!(
        runde.iter().all(|(_, delivery)| matches!(
            delivery,
            Delivery::DeadLettered(OutboxError::Critical { .. })
        )),
        "{runde:?}"
    );
    assert_eq!(zeile(&outbox, &unlesbar)?.0, "dead_letter");
    assert_eq!(zeile(&outbox, &unbekannt)?.0, "dead_letter");
    assert!(outbox.deliver_open(&dispatcher)?.is_empty());
    Ok(())
}

#[expect(
    clippy::panic_in_result_fn,
    reason = "Test: die assert! sind die Spezifikation dieses Verhaltens"
)]
#[test]
fn ein_transient_scheiternder_commit_wird_bis_zur_hoechstzahl_zugestellt_und_dann_ausgesondert()
-> rusqlite::Result<()> {
    let outbox = SqliteOutbox::open_in_memory()?;
    let mut dispatcher = Dispatcher::new();
    let versuche = registriere::<JederInhalt>(
        &mut dispatcher,
        &outbox,
        GIFT,
        Ausgang::CommitScheitertBis(u32::MAX),
    );
    let gift = outbox.enqueue(GIFT, "zeiger-1")?;

    // Die ersten Versuche: Retry, die Nachricht bleibt offen, der Versuch ist gezählt.
    for versuch in 1..MAX_DELIVERY_ATTEMPTS {
        let runde = outbox.deliver_open(&dispatcher)?;
        assert!(
            matches!(
                runde.as_slice(),
                [(_, Delivery::Redelivery(OutboxError::Retry { .. }))]
            ),
            "Versuch {versuch}: noch erneut zuzustellen: {runde:?}"
        );
        assert_eq!(zeile(&outbox, &gift)?, ("open".to_string(), versuch));
    }

    // Der letzte erlaubte Versuch: ausgesondert.
    let letzte = outbox.deliver_open(&dispatcher)?;
    assert!(
        matches!(
            letzte.as_slice(),
            [(_, Delivery::DeadLettered(OutboxError::Retry { .. }))]
        ),
        "nach {MAX_DELIVERY_ATTEMPTS} Versuchen ausgesondert: {letzte:?}"
    );
    assert_eq!(
        zeile(&outbox, &gift)?,
        ("dead_letter".to_string(), MAX_DELIVERY_ATTEMPTS)
    );
    assert!(outbox.deliver_open(&dispatcher)?.is_empty());
    assert_eq!(versuche.get(), MAX_DELIVERY_ATTEMPTS);
    Ok(())
}

#[expect(
    clippy::panic_in_result_fn,
    reason = "Test: die assert! sind die Spezifikation dieses Verhaltens"
)]
#[test]
fn ein_commit_der_beim_zweiten_versuch_gelingt_wird_verarbeitet_und_zaehlt_beide_versuche()
-> rusqlite::Result<()> {
    let outbox = SqliteOutbox::open_in_memory()?;
    let mut dispatcher = Dispatcher::new();
    registriere::<JederInhalt>(
        &mut dispatcher,
        &outbox,
        GIFT,
        Ausgang::CommitScheitertBis(1),
    );
    let id = outbox.enqueue(GIFT, "zeiger-1")?;

    let erste = outbox.deliver_open(&dispatcher)?;
    let zweite = outbox.deliver_open(&dispatcher)?;

    assert!(matches!(erste.as_slice(), [(_, Delivery::Redelivery(_))]));
    assert!(matches!(zweite.as_slice(), [(_, Delivery::Processed)]));
    assert_eq!(zeile(&outbox, &id)?, ("processed".to_string(), 2));
    assert!(outbox.dead_letters()?.is_empty());
    Ok(())
}

#[expect(
    clippy::panic_in_result_fn,
    reason = "Test: die assert! sind die Spezifikation dieses Verhaltens"
)]
#[test]
fn ausgesonderte_nachrichten_sind_mit_grund_abfragbar_und_der_grund_traegt_keinen_rohwert_und_keinen_freitext()
-> rusqlite::Result<()> {
    // Alle drei Wege ins Aussondern, jede Nachricht mit einem Rohwert im `partition_key` und
    // Freitext im `content`: der Slice scheitert, die Hülle trägt nicht, der Commit scheitert dauerhaft.
    let outbox = SqliteOutbox::open_in_memory()?;
    let mut dispatcher = Dispatcher::new();
    registriere::<JederInhalt>(&mut dispatcher, &outbox, "Slice", Ausgang::Terminal);
    registriere::<KeinInhalt>(&mut dispatcher, &outbox, "Huelle", Ausgang::Gelingt);
    registriere::<JederInhalt>(
        &mut dispatcher,
        &outbox,
        "Commit",
        Ausgang::CommitScheitertBis(u32::MAX),
    );
    for message_type in ["Slice", "Huelle", "Commit"] {
        reihe_mit_rohwert_ein(&outbox, message_type)?;
    }
    for _ in 0..MAX_DELIVERY_ATTEMPTS {
        outbox.deliver_open(&dispatcher)?;
    }

    let ausgesondert = outbox.dead_letters()?;

    let typen: Vec<&str> = ausgesondert.iter().map(DeadLetter::message_type).collect();
    assert_eq!(
        typen,
        ["Slice", "Huelle", "Commit"],
        "in Einreih-Reihenfolge"
    );
    // Positiv: der Grund ist da und benennt den Ort des Bruchs — sonst bliebe die Leak-Prüfung auch
    // dann grün, wenn gar nichts abgelegt würde.
    let gruende: Vec<&str> = ausgesondert.iter().map(DeadLetter::reason).collect();
    assert_eq!(
        gruende.first().copied(),
        Some("im Slice: Pflichtfeld betrag fehlt")
    );
    assert!(
        gruende
            .get(1)
            .is_some_and(|grund| grund.contains("Nachrichten-Hülle"))
    );
    assert!(
        gruende
            .get(2)
            .is_some_and(|grund| grund.starts_with("beim Commit:"))
    );
    assert_eq!(
        ausgesondert
            .iter()
            .map(DeadLetter::attempts)
            .collect::<Vec<_>>(),
        [1, 1, MAX_DELIVERY_ATTEMPTS]
    );
    // Negativ: weder der Rohwert des Schlüssels noch der Freitext des Inhalts.
    for grund in gruende {
        assert!(!grund.contains(ROHE_IBAN), "{grund}");
        assert!(!grund.contains(FREITEXT), "{grund}");
    }
    Ok(())
}

#[expect(
    clippy::panic_in_result_fn,
    reason = "Test: die assert! sind die Spezifikation dieses Verhaltens"
)]
#[test]
fn eine_gesunde_nachricht_hinter_einer_vergifteten_wird_weiter_zugestellt() -> rusqlite::Result<()>
{
    let outbox = SqliteOutbox::open_in_memory()?;
    let mut dispatcher = Dispatcher::new();
    registriere::<JederInhalt>(&mut dispatcher, &outbox, GIFT, Ausgang::Terminal);
    let gesund_versuche =
        registriere::<JederInhalt>(&mut dispatcher, &outbox, GESUND, Ausgang::Gelingt);
    let gift = outbox.enqueue(GIFT, "zeiger-1")?;
    let gesund = outbox.enqueue(GESUND, "zeiger-2")?;

    let runde = outbox.deliver_open(&dispatcher)?;

    assert!(
        matches!(
            runde.as_slice(),
            [
                (vorn, Delivery::DeadLettered(_)),
                (hinten, Delivery::Processed),
            ] if vorn.id == gift && hinten.id == gesund
        ),
        "die Kette bleibt nicht stehen: {runde:?}"
    );
    assert_eq!(gesund_versuche.get(), 1);
    // Und auch der nächste Lauf findet nichts mehr, das ihn aufhielte.
    let gesund_spaeter = outbox.enqueue(GESUND, "zeiger-3")?;
    let spaeter = outbox.deliver_open(&dispatcher)?;
    assert!(
        matches!(spaeter.as_slice(), [(message, Delivery::Processed)] if message.id == gesund_spaeter),
        "{spaeter:?}"
    );
    Ok(())
}

#[expect(
    clippy::panic_in_result_fn,
    reason = "Test: die assert! sind die Spezifikation dieses Verhaltens"
)]
#[test]
fn ein_ausgesonderter_status_ohne_grund_ist_unkonstruierbar() -> rusqlite::Result<()> {
    // Die Tabellen-CHECKs machen die illegalen Kombinationen unkonstruierbar: ausgesondert ohne Grund
    // und geackt ohne Ack-Stempel lehnt SQLite ab.
    let outbox = SqliteOutbox::open_in_memory()?;
    let id = outbox.enqueue(GIFT, "zeiger-1")?;

    let ohne_grund = outbox.with_connection(|conn| {
        conn.execute(
            "UPDATE outbox SET status = 'dead_letter' WHERE id = ?1;",
            [&id],
        )
    });
    let ohne_stempel = outbox.with_connection(|conn| {
        conn.execute(
            "UPDATE outbox SET status = 'processed' WHERE id = ?1;",
            [&id],
        )
    });

    assert!(
        matches!(ohne_grund, Err(rusqlite::Error::SqliteFailure(..))),
        "ausgesondert ohne Grund: {ohne_grund:?}"
    );
    assert!(
        matches!(ohne_stempel, Err(rusqlite::Error::SqliteFailure(..))),
        "geackt ohne Ack-Stempel: {ohne_stempel:?}"
    );
    assert_eq!(zeile(&outbox, &id)?.0, "open");
    Ok(())
}

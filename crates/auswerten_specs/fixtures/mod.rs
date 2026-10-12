//! Die **spec-lokalen** Fixtures/Helfer des auswerten-Slice — Input-Builder, Expected-Builder und
//! Test-Consts, die die nackten Tests in `tests/` per Modul-Include (`#[path = …] mod fixtures;`)
//! hereinziehen. Erwartungsdaten sind test-seitig und gehören darum hierher, nicht in die
//! Adapter-`TestApi`.
//!
//! Der Slice hat **mehrere** Spec-Dateien — eine je Reichweite (ADR-0013, Entscheidung E), plus die
//! thematisch ausgegliederten Fingerabdruck-Specs — und damit je ein Test-Binary, in das dieses
//! Modul **vollständig** hineinkompiliert wird. Jedes nutzt naturgemäß nur seinen Teil, was ohne
//! das `allow` in jedem Binary `dead_code` für die Helfer der jeweils anderen meldete. Das ist der
//! bekannte Preis geteilter Test-Helfer, kein Hinweis auf ungenutzten Code: was hier steht, hat in
//! mindestens einer der Specs einen Aufrufer.
#![allow(
    dead_code,
    reason = "geteiltes Fixture-Modul: es wird in jedes Spec-Binary vollstaendig hineinkompiliert, \
              jedes nutzt nur seinen Teil"
)]

use auswerten_adapter::{
    AccountCashflowRow, AnalysisWindowRow, BookingDirection, BookingRecipient, CategoryRequestRow,
    CategoryRow, CostDriverRow, CounterAccountRow, DriverRankRow, EvaluationTestApi, FeeRow,
    HistoryTestApi, InternalTransferRow, MonthlyCashflowRow, OccurrenceRow, OwnAccountRow,
    OwnAccountsConfig, PriceIncreaseRow, RecurrenceRhythm, RecurrenceRow, WindowBookingRow,
};

/// Der **opake Seed-Marker** des Normalfalls: Fenster und Eigene-Konten-Config sind mit **demselben**
/// Seed tokenisiert, also tragen beide dieses Etikett. Es ist **nicht** der Seed — nur ein nicht
/// umkehrbares Kennzeichen seiner Identität, das io beiden Seiten mitgibt (ADR-0010: der Seed selbst
/// erreicht diesen Slice nie).
pub(crate) const SEED_MARKER: &str = "marker-a";

/// Das **belastete** Konto der Buchungen im Normalfall — das Auftragskonto als `KONTO_…`-Token, wie
/// `obfuskieren` es abgelegt hat. Jede Buchung steht auf genau einem Konto; welches es ist,
/// interessiert erst die Gebühren-Sicht, die je Konto trennt.
pub(crate) const EIGENES_KONTO: &str = "KONTO_MEIN";

/// Ein **zweites** eigenes Konto — nur dort gebraucht, wo nachzuweisen ist, dass Gebühren je Konto
/// getrennt geführt werden statt in einen Topf zu fallen.
pub(crate) const ZWEITES_KONTO: &str = "KONTO_ZWEIT";

/// Ein Buchungstext, den die Bank für ihre **eigenen Entgelte** setzt — das Signal, an dem die
/// Gebühren-Klassifikation greift (Issue 0006). Erfunden wie alle Testdaten, aber aus demselben
/// Vokabular wie die CSV-Fixtures.
pub(crate) const ENTGELT_TEXT: &str = "ENTGELTABSCHLUSS";

/// Die opake ID des eingefrorenen Auswertungsfensters, auf die das konsumierte
/// `FensterGeschlossen` zeigt — die **Ablage von `fenstern`** hat sie vergeben (uniformes
/// ID-Handoff, ADR-0012: Message = Zeiger, Tabelle = Wahrheit).
pub(crate) const FENSTER: &str = "fenster-1";

// Buchungstage als sortierbare Tag-Stamps (Tage seit Epoche, ADR-0013 Entscheidung C). Die
// Kalender-Sicht dahinter steht im Namen — genau sie rekonstruiert dieser Slice beim Bucketing.
/// 2024-01-15
pub(crate) const JAN_15: i64 = 19_737;
/// 2024-01-31 — der letzte Tag des Januar (die Monatsgrenze von unten).
pub(crate) const JAN_31: i64 = 19_753;
/// 2024-02-01 — der erste Tag des Februar (die Monatsgrenze von oben).
pub(crate) const FEB_01: i64 = 19_754;
/// 2024-02-29 — Schalttag.
pub(crate) const FEB_29: i64 = 19_782;
/// 2024-03-05
pub(crate) const MAR_05: i64 = 19_787;

/// Eine TestApi mit leerer Fenster-Ablage und **leerer** Eigene-Konten-Config unter dem
/// Normalfall-Seed-Marker — die Lage „ich führe keine eigenen Konten", in der jedes Gegenkonto
/// fremd ist. Wer die Eigen-Menge braucht, arrangiert sie mit `with_own_accounts`.
pub(crate) fn api() -> EvaluationTestApi {
    EvaluationTestApi::new().with_own_accounts(eigene_konten(&[]))
}

/// Ein eingefrorenes Auswertungsfenster in Euro, wie der Lese-Seam es zur ID liefert — unter dem
/// **Normalfall-Seed-Marker**.
pub(crate) fn fenster(bookings: &[WindowBookingRow]) -> AnalysisWindowRow {
    fenster_mit_marker(SEED_MARKER, bookings)
}

/// Dasselbe Fenster unter einem **frei gewählten** Seed-Marker — für den Fall, dass Fenster und
/// Config aus verschiedenen Tokenisierungen stammen.
pub(crate) fn fenster_mit_marker(
    seed_marker: &str,
    bookings: &[WindowBookingRow],
) -> AnalysisWindowRow {
    AnalysisWindowRow {
        currency: "EUR".to_string(),
        seed_marker: seed_marker.to_string(),
        bookings: bookings.to_vec(),
    }
}

/// Ein **Abfluss** an einem Buchungstag (Betrag vorzeichenlos in Minor Units — das Vorzeichen lebt allein
/// in der Richtung, ADR-0005) — **ohne Gegenpartei** und **ohne Buchungstext**, also keine erkennbare
/// Gebühr (dafür `entgelt`).
pub(crate) fn ausgang(booking_day: i64, minor_units: u64) -> WindowBookingRow {
    WindowBookingRow {
        booking_day,
        amount_minor_units: minor_units,
        direction: BookingDirection::Outgoing,
        ordering_account: EIGENES_KONTO.to_string(),
        ..WindowBookingRow::default()
    }
}

/// Eine **Entgeltzeile**: der Abfluss, den die Bank selbst einzieht — ohne Gegenpartei (sie bucht
/// direkt aufs Konto) und mit dem Buchungstext, an dem die Klassifikation ihn erkennt.
pub(crate) fn entgelt(booking_day: i64, minor_units: u64) -> WindowBookingRow {
    entgelt_auf(booking_day, minor_units, EIGENES_KONTO)
}

/// Dieselbe Entgeltzeile auf einem **frei gewählten** Konto — für den Nachweis, dass Gebühren je
/// belastetem Konto getrennt geführt werden.
pub(crate) fn entgelt_auf(booking_day: i64, minor_units: u64, konto: &str) -> WindowBookingRow {
    WindowBookingRow {
        booking_day,
        amount_minor_units: minor_units,
        direction: BookingDirection::Outgoing,
        ordering_account: konto.to_string(),
        booking_text: Some(ENTGELT_TEXT.to_string()),
        ..WindowBookingRow::default()
    }
}

/// Ein **Zufluss** an einem Buchungstag — ebenfalls ohne Gegenpartei.
pub(crate) fn eingang(booking_day: i64, minor_units: u64) -> WindowBookingRow {
    WindowBookingRow {
        booking_day,
        amount_minor_units: minor_units,
        direction: BookingDirection::Incoming,
        ordering_account: EIGENES_KONTO.to_string(),
        ..WindowBookingRow::default()
    }
}

/// Ein Abfluss an eine **sichere Firma** — so, wie `obfuskieren` sie ablegt: die Gläubiger-ID als
/// öffentlicher Anker, der Firmenname im Klartext (bewusstes Kategorie-Signal) und der
/// Verwendungszweck als obfuskierter Freitext.
pub(crate) fn firma(
    booking_day: i64,
    minor_units: u64,
    creditor_id: &str,
    name: &str,
    purpose: Option<&str>,
) -> WindowBookingRow {
    WindowBookingRow {
        booking_day,
        amount_minor_units: minor_units,
        direction: BookingDirection::Outgoing,
        recipient: BookingRecipient::Company {
            name: Some(name.to_string()),
            creditor_id: creditor_id.to_string(),
        },
        purpose: purpose.map(str::to_string),
        counter_account: None,
        ordering_account: EIGENES_KONTO.to_string(),
        booking_text: None,
    }
}

/// Ein Abfluss an eine Firma **ohne geführten Namen** — erreichbar, weil `recipient_name` von
/// `einlesen` an durchweg optional ist, die Gläubiger-ID an einer Firma dagegen Pflicht.
pub(crate) fn firma_ohne_namen(
    booking_day: i64,
    minor_units: u64,
    creditor_id: &str,
    purpose: Option<&str>,
) -> WindowBookingRow {
    WindowBookingRow {
        booking_day,
        amount_minor_units: minor_units,
        direction: BookingDirection::Outgoing,
        recipient: BookingRecipient::Company {
            name: None,
            creditor_id: creditor_id.to_string(),
        },
        purpose: purpose.map(str::to_string),
        counter_account: None,
        ordering_account: EIGENES_KONTO.to_string(),
        booking_text: None,
    }
}

/// Ein Abfluss an eine **fail-safe Privatperson** — sie trägt **nur** ihr seed-basiertes Pseudonym
/// (`PERSON_…`), das `obfuskieren` vergeben hat. Ein anderer Seed ergibt stromaufwärts ein anderes
/// Pseudonym; dieser Slice sieht den Seed nie (ADR-0010).
pub(crate) fn person(booking_day: i64, minor_units: u64, pseudonym: &str) -> WindowBookingRow {
    WindowBookingRow {
        booking_day,
        amount_minor_units: minor_units,
        direction: BookingDirection::Outgoing,
        recipient: BookingRecipient::Person {
            pseudonym: pseudonym.to_string(),
        },
        purpose: None,
        counter_account: None,
        ordering_account: EIGENES_KONTO.to_string(),
        booking_text: None,
    }
}

/// Ein Abfluss auf ein **Gegenkonto**, das nur als Token vorliegt (`KONTO_…`) — so, wie
/// `obfuskieren` es an der Datenschutz-Grenze abgelegt hat. Genau dieses Token prüft die Auflösung
/// gegen die Eigen-Menge; eine rohe IBAN erreicht diesen Slice nie (ADR-0013, Entscheidung F).
pub(crate) fn gegenkonto(booking_day: i64, minor_units: u64, token: &str) -> WindowBookingRow {
    WindowBookingRow {
        booking_day,
        amount_minor_units: minor_units,
        direction: BookingDirection::Outgoing,
        counter_account: Some(token.to_string()),
        ordering_account: EIGENES_KONTO.to_string(),
        ..WindowBookingRow::default()
    }
}

/// Die **token-orientierte Eigene-Konten-Config**, wie sie dem Handler als **Wert** hereingereicht
/// wird: je eigenem Konto sein `KONTO_…`-Token (vom Loader mit **demselben** Seed tokenisiert wie
/// die Gegenkonto-Token) und seine Rolle. Erfundene Werte — die echte Config bleibt HITL (0002).
pub(crate) fn eigene_konten(accounts: &[(&str, &str)]) -> OwnAccountsConfig {
    eigene_konten_mit_marker(SEED_MARKER, accounts)
}

/// Dieselbe Config unter einem **frei gewählten** Seed-Marker — so lässt sich eine Eigen-Menge
/// arrangieren, die aus einer **anderen** Tokenisierung stammt als das Fenster.
pub(crate) fn eigene_konten_mit_marker(
    seed_marker: &str,
    accounts: &[(&str, &str)],
) -> OwnAccountsConfig {
    OwnAccountsConfig {
        seed_marker: seed_marker.to_string(),
        accounts: accounts
            .iter()
            .map(|(token, role)| OwnAccountRow {
                token: (*token).to_string(),
                role: (*role).to_string(),
            })
            .collect(),
    }
}

/// Ein erwartetes **eigenes** Gegenkonto samt seiner Rolle — die Rolle trägt die Deutung (ein
/// Zufluss aufs Spar-Konto ist Sparen, nicht Konsum).
pub(crate) fn eigen(role: &str) -> CounterAccountRow {
    CounterAccountRow::Own {
        role: role.to_string(),
    }
}

/// Ein erwarteter **Monats-Bucket** des Cashflows: Zu- und Abfluss getrennt und je vorzeichenlos.
/// **Genettet** — eine Umbuchung zwischen eigenen Konten steht nicht hier, sondern in `umgebucht`.
pub(crate) fn monat(year: i64, month: i64, incoming: u64, outgoing: u64) -> MonthlyCashflowRow {
    MonthlyCashflowRow {
        year,
        month,
        incoming_minor_units: incoming,
        outgoing_minor_units: outgoing,
    }
}

/// Ein erwarteter **Umbuchungs-Bucket**: dieselbe Form wie ein Monats-Bucket, dazu die Rolle des
/// **Gegenkontos** — ein Abfluss unter `spar` ging aufs Sparkonto, ein Zufluss unter `spar` kam von
/// dort. Was hier steht, fehlt im `monat` desselben Zeitraums; zusammen ergeben sie das Brutto.
pub(crate) fn umgebucht(
    year: i64,
    month: i64,
    role: &str,
    incoming: u64,
    outgoing: u64,
) -> InternalTransferRow {
    InternalTransferRow {
        year,
        month,
        role: role.to_string(),
        incoming_minor_units: incoming,
        outgoing_minor_units: outgoing,
    }
}

/// Ein erwarteter **Gebühren-Bucket**: dieselbe Form wie ein Monats-Bucket, dazu das **belastete
/// Konto** als Token. Anders als bei `umgebucht` steht, was hier steht, **auch** im `monat`
/// desselben Zeitraums — eine Gebühr ist echter Abfluss und wird nur zusätzlich herausgehoben.
pub(crate) fn gebuehr(year: i64, month: i64, konto: &str, incoming: u64, outgoing: u64) -> FeeRow {
    FeeRow {
        year,
        month,
        account: konto.to_string(),
        incoming_minor_units: incoming,
        outgoing_minor_units: outgoing,
    }
}

/// Ein **Zufluss von einem Gegenkonto**, das nur als Token vorliegt — die Gegenrichtung zu
/// `gegenkonto`: Geld, das von dort kam. Erst mit ihr lässt sich prüfen, dass das Netting **beide**
/// Richtungen erfasst und nicht nur den Abfluss.
pub(crate) fn gegenkonto_eingang(
    booking_day: i64,
    minor_units: u64,
    token: &str,
) -> WindowBookingRow {
    WindowBookingRow {
        booking_day,
        amount_minor_units: minor_units,
        direction: BookingDirection::Incoming,
        counter_account: Some(token.to_string()),
        ordering_account: EIGENES_KONTO.to_string(),
        ..WindowBookingRow::default()
    }
}

// --- Die fenster-übergreifende Reichweite (Issue 0043) -------------------------

/// Die Gläubiger-ID der wiederkehrenden Buchungsart, an der die fenster-übergreifenden Specs
/// arbeiten — sie ist der **Schlüssel**, unter dem die Vorkommen über die Fenster hinweg
/// zusammenfinden.
pub(crate) const HAENDLER: &str = "DE98ZZZ09999999999";

/// Eine TestApi der **zweiten** Reichweite mit leerer Fenster-Ablage und **leerer**
/// Eigene-Konten-Config unter dem Normalfall-Seed-Marker — jedes Gegenkonto ist dann fremd. Die
/// Config braucht diese Reichweite für das **Netting der Wiederkehr** (Issue 0005): ein Dauerauftrag
/// aufs eigene Konto ist kein Kosten-Treiber.
pub(crate) fn historie_api() -> HistoryTestApi {
    HistoryTestApi::new().with_own_accounts(eigene_konten(&[]))
}

/// Eine Buchung **derselben wiederkehrenden Art** an einem Buchungstag — derselbe Händler, also
/// derselbe Fingerabdruck, egal in welchem Fenster sie liegt.
pub(crate) fn wiederkehrend(booking_day: i64, minor_units: u64) -> WindowBookingRow {
    firma(booking_day, minor_units, HAENDLER, "Netflix", None)
}

/// Dieselbe wiederkehrende Art, aber als **Zufluss** — eine Rückerstattung desselben Händlers. Sie
/// trägt denselben Fingerabdruck: die *Art* der Buchung hängt an der Gegenpartei, nicht an der
/// Richtung.
pub(crate) fn eingang_wiederkehrend(booking_day: i64, minor_units: u64) -> WindowBookingRow {
    WindowBookingRow {
        direction: BookingDirection::Incoming,
        ..wiederkehrend(booking_day, minor_units)
    }
}

/// Ein erwartetes **Vorkommen** auf der Zeitachse des Laufs: der Fingerabdruck der Buchungs*art*
/// plus das, was Rhythmus und Betragsvergleich brauchen — Tag, Betrag und Richtung.
pub(crate) fn vorkommen(
    fingerprint: &str,
    booking_day: i64,
    minor_units: u64,
    direction: BookingDirection,
) -> OccurrenceRow {
    OccurrenceRow {
        fingerprint: fingerprint.to_string(),
        booking_day,
        amount_minor_units: minor_units,
        direction,
    }
}

// --- Die Wiederkehr-Erkennung (Issue 0005) -------------------------------------

// Ein **monatlicher** Takt über ein halbes Jahr, jeweils am 15. — 2024 (Tage seit Epoche).
/// 2024-02-15
pub(crate) const FEB_15: i64 = 19_768;
/// 2024-03-15
pub(crate) const MAR_15: i64 = 19_797;
/// 2024-04-15
pub(crate) const APR_15: i64 = 19_828;
/// 2024-05-15
pub(crate) const MAI_15: i64 = 19_858;
/// 2024-06-15
pub(crate) const JUN_15: i64 = 19_889;

// Ein **jährlicher** Takt am Monatsanfang — die Lage, die nur die Sicht über **alle** Fenster belegt.
/// 2023-03-01
pub(crate) const MAERZ_2023: i64 = 19_417;
/// 2024-03-01 (366 Tage später — 2024 ist ein Schaltjahr)
pub(crate) const MAERZ_2024: i64 = 19_783;
/// 2025-03-03 (ein Montag: der 1. fiel aufs Wochenende, die Lastschrift rutschte)
pub(crate) const MAERZ_2025: i64 = 20_150;

/// Die Gläubiger-ID des **Zahlungsintermediärs** PayPal — eine der in der Domäne hinterlegten
/// Durchleiter-IDs (öffentlich, kein personenbezogener Wert).
pub(crate) const PAYPAL: &str = "LU96ZZZ0000000000000000058";

/// Das Konto-Token eines **eigenen Sparkontos** — erfunden, wie alle Token hier.
pub(crate) const SPARKONTO: &str = "KONTO_SPAR";

/// Eine Abbuchung **über einen Durchleiter**: die Gegenpartei ist PayPal, der eigentliche Händler
/// steht nur im Verwendungszweck — samt einer **je Buchung wechselnden** Referenznummer, wie sie
/// echte PayPal-Zwecke tragen.
pub(crate) fn ueber_paypal(booking_day: i64, minor_units: u64, purpose: &str) -> WindowBookingRow {
    firma(
        booking_day,
        minor_units,
        PAYPAL,
        "PayPal Europe S.a.r.l. et Cie S.C.A.",
        Some(purpose),
    )
}

/// Ein **Dauerauftrag** an eine Privatperson auf ein Gegenkonto, das nur als Token vorliegt — je
/// nach Eigene-Konten-Config eine Umbuchung (das Token ist meines) oder ein echter Abfluss.
pub(crate) fn dauerauftrag(booking_day: i64, minor_units: u64, konto: &str) -> WindowBookingRow {
    WindowBookingRow {
        counter_account: Some(konto.to_string()),
        ..person(booking_day, minor_units, "PERSON_7")
    }
}

/// Ein erwarteter **wiederkehrender Abfluss** ohne Preissteigerung, belastet auf dem Normalfall-Konto.
pub(crate) fn wiederkehr(
    fingerprint: &str,
    rhythm: RecurrenceRhythm,
    typical: u64,
    annual: u64,
) -> RecurrenceRow {
    RecurrenceRow {
        fingerprint: fingerprint.to_string(),
        rhythm,
        typical_amount_minor_units: typical,
        annual_amount_minor_units: annual,
        price_increase: None,
        charged_account: EIGENES_KONTO.to_string(),
    }
}

/// Eine erwartete **Preissteigerung**: ab dem Stichtag `since` wird `after` statt `before` abgebucht.
pub(crate) fn preissteigerung(since: i64, before: u64, after: u64) -> PriceIncreaseRow {
    PriceIncreaseRow {
        since_day: since,
        before_minor_units: before,
        after_minor_units: after,
    }
}

// --- Das Kategorisierer-Orakel (Issue 0044) ------------------------------------

/// Der **Fingerabdruck** einer Firmen-Buchungsart, so wie die Domäne ihn bildet — der Schlüssel,
/// auf dem die Anfrage dedupliziert und die Antwort zugeordnet wird. Die Specs schreiben ihn
/// **selbst** aus, statt ihn vom Slice zu erfragen: der Schlüssel ist Teil der beobachteten
/// Zusage, nicht ein Zwischenwert, den man ihm glauben müsste.
pub(crate) fn fingerabdruck(creditor_id: &str) -> String {
    format!("COMPANY:{creditor_id}")
}

/// Eine erwartete **Anfrage ans Orakel** — das Egress-DTO: der Fingerabdruck als Schlüssel und der
/// obfuskierte Text als das, woran das LLM die Kategorie erraten soll (CONTEXT.md „Kategorie":
/// Empfänger + Verwendungszweck). Mehr steht nicht drin; ein struktureller Identifikator kann hier
/// gar nicht auftauchen.
pub(crate) fn anfrage(fingerprint: &str, text: &str) -> CategoryRequestRow {
    CategoryRequestRow {
        fingerprint: fingerprint.to_string(),
        text: text.to_string(),
    }
}

/// Eine erwartete **zugeordnete Kategorie** im abgelegten Ergebnis — die Antwort des Orakels,
/// verschlüsselt auf den **Fingerabdruck**, nie auf den Text.
pub(crate) fn kategorie(fingerprint: &str, category: &str) -> CategoryRow {
    CategoryRow {
        fingerprint: fingerprint.to_string(),
        category: category.to_string(),
    }
}

// --- Der Aderlass: Cashflow je Konto, Pareto-Ranking, Spar-Kandidaten (Issue 0007) ---

/// Die Gläubiger-ID eines **zweiten** Händlers — erfunden. Er steht neben dem wiederkehrenden
/// `HAENDLER`, damit das Ranking zwei Arten gegeneinander ordnen kann.
pub(crate) const ZWEITER_HAENDLER: &str = "DE11ZZZ01111111111";

/// Ein Abfluss an den **zweiten** Händler — eine Firma mit eigener Gläubiger-ID, also eine eigene
/// Buchungsart und damit ein eigener Kostentreiber.
pub(crate) fn zweiter_haendler(booking_day: i64, minor_units: u64) -> WindowBookingRow {
    firma(
        booking_day,
        minor_units,
        ZWEITER_HAENDLER,
        "Fitnessstudio Fiktiv",
        None,
    )
}

/// Ein erwarteter **Aderlass-Bucket** eines eigenen Kontos: Konto-Token, seine Rolle laut Config
/// (`None`, wenn die Config es nicht führt), der Monat und beide vorzeichenlosen Spalten.
pub(crate) fn konto_monat(
    konto: &str,
    rolle: Option<&str>,
    (year, month): (i64, i64),
    incoming: u64,
    outgoing: u64,
) -> AccountCashflowRow {
    AccountCashflowRow {
        account: konto.to_string(),
        role: rolle.map(str::to_string),
        year,
        month,
        incoming_minor_units: incoming,
        outgoing_minor_units: outgoing,
    }
}

/// Der erwartete Kostentreiber einer **Buchungsart**, über ihren Fingerabdruck.
pub(crate) fn art(fingerprint: &str) -> CostDriverRow {
    CostDriverRow::Kind {
        fingerprint: fingerprint.to_string(),
    }
}

/// Der erwartete Kostentreiber der **Gebühren eines Kontos**.
pub(crate) fn gebuehren_von(konto: &str) -> CostDriverRow {
    CostDriverRow::Fee {
        account: konto.to_string(),
    }
}

/// Ein erwarteter **Platz im Ranking** ohne Wiederkehr: Abfluss im Lauf, Zahl der Abflüsse, Anteil
/// und kumulierter Anteil in Basispunkten (10 000 = 100 %) und der Jahresbetrag.
pub(crate) fn platz(
    driver: CostDriverRow,
    (outflow, bookings): (u64, usize),
    (share, cumulative): (u32, u32),
    annual: u64,
) -> DriverRankRow {
    DriverRankRow {
        driver,
        outflow_minor_units: outflow,
        bookings,
        share_basis_points: share,
        cumulative_share_basis_points: cumulative,
        annual_amount_minor_units: annual,
        rhythm: None,
        price_increase: None,
    }
}

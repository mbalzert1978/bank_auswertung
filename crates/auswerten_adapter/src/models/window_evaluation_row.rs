use auswerten_domain::{CounterAccount, WindowEvaluation};

use crate::models::CategoryRow;
use crate::models::account_role_vocabulary::role_label;

/// Das **abgelegte Pro-Fenster-Ergebnis** als Zeile der eigenen Tabelle des Slice — das Ausgangs-DTO,
/// das die [`RepositoryBridge`](crate::adapter::RepositoryBridge) in den Speicher-Seam staged. Es
/// trägt den **Fremdschlüssel** auf sein Quell-Fenster (ADR-0012: Aggregate referenzieren
/// Fremd-Entitäten nur per ID), die eine Währung und die Monats-Buckets des Cashflows. Seine
/// **eigene** ID vergibt die Persistenz beim Ablegen — sie steht darum nicht hier drin.
///
/// **Datenschutz:** stromabwärts der Obfuskierungs-Grenze; die Zeile trägt nur opake IDs, eine
/// Währung, Kalender-Zahlen und Beträge — keinen strukturellen Identifikator und keinen Freitext.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowEvaluationRow {
    pub window_id: String,
    pub currency: String,
    /// Die Monats-Buckets in **chronologischer** Reihenfolge.
    pub months: Vec<MonthlyCashflowRow>,
    /// Die **distinkten** Fingerabdrücke der Buchungsarten des Fensters, in stabiler Reihenfolge —
    /// der Schlüssel, auf dem Wiederkehr-Erkennung, gelernte Korrekturen und die Deduplizierung der
    /// Kategorisierer-Anfrage später aufsetzen.
    pub fingerprints: Vec<String>,
    /// Die **distinkten** Einordnungen der Gegenkonten des Fensters, in stabiler Reihenfolge — je
    /// eigen (samt Rolle) oder fremd. Aufgelöst **auf Token**; eine IBAN steht hier so wenig wie
    /// irgendwo sonst in diesem Slice.
    pub counter_accounts: Vec<CounterAccountRow>,
    /// Was das **Orakel** zu den zuvor **unbekannten** Buchungsarten dieses Fensters beigetragen
    /// hat — je Eintrag Fingerabdruck → Kategorie, in stabiler Reihenfolge. Bereits Bekanntes steht
    /// hier **nicht**: das führt das Kategorie-Register, und es an jedes Fenster zu heften hieße,
    /// die gelernte Wahrheit zu duplizieren.
    pub categories: Vec<CategoryRow>,
}

/// Ein **eingeordnetes Gegenkonto** als Zeile — **adapter-eigener** DTO-Typ, bewusst re-deklariert
/// statt aus der Domäne re-exportiert (dieselbe Linie, die der Slice bei `BookingDirection` und
/// `BookingRecipient` zieht): kein Domänen-Internum überquert die Grenze.
///
/// Er bleibt ein **Summentyp**: „Fremd **mit** Rolle" ist auch als DTO unkonstruierbar, nicht
/// „geprüft" (ADR-0005) — ein Flag plus Optionalfeld hätte genau diesen illegalen Zustand
/// wieder aufgemacht.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CounterAccountRow {
    /// Ein **eigenes** Konto samt seiner Rolle (`giro` | `haushalt` | `spar` | `vorlauf`).
    Own { role: String },
    /// Ein **fremdes** Konto — es hat keine Rolle, und es kann keine tragen.
    Foreign,
}

/// Projiziert die Domänen-Einordnung auf das DTO. Die Rolle wird dabei auf **dasselbe Vokabular**
/// abgebildet, das die Config hereinreicht — so lesen Ein- und Ausgang dieselbe Sprache.
impl From<&CounterAccount> for CounterAccountRow {
    fn from(counter_account: &CounterAccount) -> Self {
        match counter_account {
            CounterAccount::Own(role) => CounterAccountRow::Own {
                role: role_label(*role).to_string(),
            },
            CounterAccount::Foreign => CounterAccountRow::Foreign,
        }
    }
}

/// Ein **Monats-Bucket** des Cashflows als Zeile: Jahr und Monat des *Zeitraums* (nicht des
/// Fensters!) und die beiden **vorzeichenlosen** Spalten. Das Vorzeichen lebt in der Richtung
/// (ADR-0005), darum stehen Zu- und Abfluss getrennt statt als Saldo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MonthlyCashflowRow {
    pub year: i64,
    /// Der Monat im Jahr, **1..=12** — von der Domäne per Konstruktion garantiert.
    pub month: i64,
    pub incoming_cent: u64,
    pub outgoing_cent: u64,
}

/// Projiziert das **Domänen-Aggregat** auf die Zeile — die einzige Stelle, an der es die
/// Adapter-Grenze überquert (die Repository-Bridge ruft sie vor dem Ablegen).
impl From<&WindowEvaluation> for WindowEvaluationRow {
    fn from(evaluation: &WindowEvaluation) -> Self {
        Self {
            window_id: evaluation.window().as_str().to_string(),
            currency: evaluation.currency().as_str().to_string(),
            months: evaluation
                .months()
                .iter()
                .map(|cashflow| MonthlyCashflowRow {
                    year: cashflow.month().year(),
                    month: cashflow.month().month_of_year(),
                    incoming_cent: cashflow.incoming().cent(),
                    outgoing_cent: cashflow.outgoing().cent(),
                })
                .collect(),
            fingerprints: evaluation
                .fingerprints()
                .iter()
                .map(|fingerprint| fingerprint.as_str().to_string())
                .collect(),
            counter_accounts: evaluation
                .counter_accounts()
                .iter()
                .map(CounterAccountRow::from)
                .collect(),
            categories: evaluation
                .categories()
                .iter()
                .map(CategoryRow::from)
                .collect(),
        }
    }
}

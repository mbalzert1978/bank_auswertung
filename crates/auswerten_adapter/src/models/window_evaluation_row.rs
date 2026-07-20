use auswerten_domain::WindowEvaluation;

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
        }
    }
}

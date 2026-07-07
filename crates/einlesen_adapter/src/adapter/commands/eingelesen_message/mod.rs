use serde::Serialize;
use shared_kernel::OutboxCommand;

use crate::abstractions::Kalendertag;
use crate::eingelesen::MESSAGE_TYPE;
use crate::zeile::{Richtung, Zeile};

/// Das serialisierbare **Wire-DTO** des `Eingelesen`-Commands: die flache, serde-fähige
/// Form, die io in die Outbox schreibt und der nächste Slice mit derselben Konvention wieder
/// liest ([`outbox_payload`](shared_kernel::outbox_payload): camelCase). Die Domänen-Wert-Typen
/// der [`Zeile`] werden dabei auf Primitive abgeflacht (die Domäne bleibt serde-frei); rohe
/// Werte dürfen hier serialisieren (ADR-0008-Reframe: lokaler Outbox-Hop, kein Egress).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EingelesenMessage {
    auftragskonto: String,
    gegenkonto: Option<String>,
    betrag_cent: u64,
    richtung: &'static str,
    empfänger_name: String,
    gläubiger_id: Option<String>,
    buchungstag: String,
    valuta: String,
    verwendungszweck: Option<String>,
    buchungstext: Option<String>,
}

impl EingelesenMessage {
    /// Flacht die [`Zeile`] auf die serialisierbare Wire-Form ab (Publisher-Bridge).
    pub(crate) fn aus_zeile(zeile: &Zeile) -> Self {
        Self {
            auftragskonto: zeile.auftragskonto.clone(),
            gegenkonto: zeile.gegenkonto.clone(),
            betrag_cent: zeile.betrag_cent,
            richtung: match zeile.richtung {
                Richtung::Eingang => "Eingang",
                Richtung::Ausgang => "Ausgang",
            },
            empfänger_name: zeile.empfänger_name.clone(),
            gläubiger_id: zeile.gläubiger_id.clone(),
            buchungstag: als_iso(zeile.buchungstag),
            valuta: als_iso(zeile.valuta),
            verwendungszweck: zeile.verwendungszweck.clone(),
            buchungstext: zeile.buchungstext.clone(),
        }
    }
}

/// Flacht einen Kalendertag auf die ISO-Textform `JJJJ-MM-TT` ab.
fn als_iso(tag: Kalendertag) -> String {
    format!("{:04}-{:02}-{:02}", tag.jahr, tag.monat, tag.tag)
}

impl OutboxCommand for EingelesenMessage {
    const MESSAGE_TYPE: &'static str = MESSAGE_TYPE;

    fn partition_key(&self) -> &str {
        &self.auftragskonto
    }
}

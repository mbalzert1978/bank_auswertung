use einlesen_domain::{Kalendertag, Richtung, ZeilenAggregat};

/// Das ausgehende DTO des Slice — die **eine** Datensicht, die `einlesen` nach außen gibt
/// (an den `Zeilenspeicher` gestaget und als Nutzlast des `Eingelesen`-Commands
/// weitergereicht). Es spiegelt das `ZeilenAggregat` der Domäne, ohne einen Domänentyp
/// über die Adapter-Grenze zu lassen: die rohen Identifikatoren (IBANs, Klarname) reisen
/// hier **roh** — das Obfuskieren ist der nachgelagerte `obfuskieren`-Slice
/// ([ADR-0008](../../../docs/adr/0008-event-getriebene-pipeline-und-outbox.md)-Reframe: rohe
/// Werte dürfen lokal durch die Outbox, kein Egress; darum bewusst **kein** `Serialize`-Guard).
///
/// Die einfachen Wert-Typen `Richtung`/`Kalendertag` bleiben als Feldtypen erhalten — sie
/// sind Ubiquitous Language, kein primitives Ersatzkonstrukt (DTO spricht Domänensprache).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Zeile {
    pub auftragskonto: String,
    pub gegenkonto: Option<String>,
    pub betrag_cent: u64,
    pub richtung: Richtung,
    pub empfänger_name: String,
    pub gläubiger_id: Option<String>,
    pub buchungstag: Kalendertag,
    pub valuta: Kalendertag,
    pub verwendungszweck: Option<String>,
    pub buchungstext: Option<String>,
}

impl Zeile {
    /// Projiziert das validierte Aggregat auf das DTO — die einzige Stelle, an der ein
    /// `ZeilenAggregat` zu einer `Zeile` wird (der Handler ruft sie je gültiger Zeile).
    pub(crate) fn aus_aggregat(aggregat: &ZeilenAggregat) -> Self {
        let empfänger = aggregat.empfänger();
        Self {
            auftragskonto: aggregat.auftragskonto().as_str().to_string(),
            gegenkonto: aggregat.gegenkonto().map(|iban| iban.as_str().to_string()),
            betrag_cent: aggregat.betrag().cent(),
            richtung: aggregat.richtung(),
            empfänger_name: empfänger.name().as_str().to_string(),
            gläubiger_id: empfänger.gläubiger_id().map(|id| id.as_str().to_string()),
            buchungstag: aggregat.datum().buchungstag(),
            valuta: aggregat.datum().valuta(),
            verwendungszweck: aggregat
                .verwendungszweck()
                .map(|zweck| zweck.as_str().to_string()),
            buchungstext: aggregat
                .buchungstext()
                .map(|text| text.as_str().to_string()),
        }
    }
}

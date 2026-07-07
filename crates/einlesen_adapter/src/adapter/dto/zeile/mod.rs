use einlesen_domain::ZeilenAggregat;

use crate::abstractions::Kalendertag;

/// Die Richtung einer Buchung — **adapter-eigen** (kein Re-Export von
/// `einlesen_domain::Richtung`): dieselbe Zwei-Wert-Unterscheidung wie die Domäne, aber ein
/// eigener Typ, weil `Zeile` die Adapter-Grenze überquert und keine Domänen-Interna dorthin
/// durchreichen soll.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Richtung {
    Eingang,
    Ausgang,
}

/// Das ausgehende DTO des Slice — die **eine** Datensicht, die `einlesen` nach außen gibt (als
/// Nutzlast des `Eingelesen`-Commands). Es spiegelt das `ZeilenAggregat` der Domäne, ohne einen
/// Domänentyp über die Adapter-Grenze zu lassen: die rohen Identifikatoren (IBANs, Klarname)
/// reisen hier **roh** — das Obfuskieren ist der nachgelagerte `obfuskieren`-Slice
/// ([ADR-0008](../../../docs/adr/0008-event-getriebene-pipeline-und-outbox.md)-Reframe: rohe
/// Werte dürfen lokal durch die Outbox, kein Egress; darum bewusst **kein** `Serialize`-Guard).
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
        let buchungstag = aggregat.datum().buchungstag();
        let valuta = aggregat.datum().valuta();
        Self {
            auftragskonto: aggregat.auftragskonto().as_str().to_string(),
            gegenkonto: aggregat.gegenkonto().map(|iban| iban.as_str().to_string()),
            betrag_cent: aggregat.betrag().cent(),
            richtung: match aggregat.richtung() {
                einlesen_domain::Richtung::Eingang => Richtung::Eingang,
                einlesen_domain::Richtung::Ausgang => Richtung::Ausgang,
            },
            empfänger_name: empfänger.name().as_str().to_string(),
            gläubiger_id: empfänger.gläubiger_id().map(|id| id.as_str().to_string()),
            buchungstag: Kalendertag {
                jahr: buchungstag.jahr(),
                monat: buchungstag.monat(),
                tag: buchungstag.tag(),
            },
            valuta: Kalendertag {
                jahr: valuta.jahr(),
                monat: valuta.monat(),
                tag: valuta.tag(),
            },
            verwendungszweck: aggregat
                .verwendungszweck()
                .map(|zweck| zweck.as_str().to_string()),
            buchungstext: aggregat
                .buchungstext()
                .map(|text| text.as_str().to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::abstractions::{KalenderSeam, Kalendertag as AdapterKalendertag};

    use super::{Kalendertag, Richtung, Zeile, ZeilenAggregat};
    use crate::rohsatz::Rohsatz;

    fn feld(wert: &str) -> Option<String> {
        Some(wert.to_string())
    }

    /// Ein vollständig gültiger Rohsatz für die Mapping-Prüfung — direkter Feld-Bauer statt
    /// Fluent-Overrides (die gehören in die Specs, nicht hierher).
    fn gültiger_rohsatz() -> Rohsatz {
        Rohsatz {
            auftragskonto: feld("DE89370400440532013000"),
            buchungstag: feld("13.06.2024"),
            valuta: feld("14.06.2024"),
            betrag: feld("-50,00"),
            währung: feld("EUR"),
            empfänger_name: feld("EDEKA SAGT DANKE"),
            gläubiger_id: feld("DE98ZZZ09999999999"),
            gegenkonto: feld("DE11520513735120710131"),
            verwendungszweck: feld("Einkauf"),
            buchungstext: feld("LASTSCHRIFT"),
        }
    }

    fn parse_deutsches_datum(roh: &str) -> Option<AdapterKalendertag> {
        let mut teile = roh.split('.');
        let tag: u8 = teile.next()?.parse().ok()?;
        let monat: u8 = teile.next()?.parse().ok()?;
        let jahr: u16 = teile.next()?.parse().ok()?;
        Some(AdapterKalendertag { jahr, monat, tag })
    }

    struct DeutscherKalenderSeam;

    impl KalenderSeam for DeutscherKalenderSeam {
        fn deute(&self, roh: &str) -> Option<AdapterKalendertag> {
            parse_deutsches_datum(roh)
        }
    }

    #[test]
    fn aus_aggregat_übernimmt_alle_felder_originalgetreu() {
        let kalender = crate::kalender_bridge::KalenderBridge {
            seam: &DeutscherKalenderSeam,
        };
        let aggregat = ZeilenAggregat::aus_rohzeile(&gültiger_rohsatz(), &kalender)
            .expect("gültige Zeile");

        let zeile = Zeile::aus_aggregat(&aggregat);

        assert_eq!(zeile.auftragskonto, "DE89370400440532013000");
        assert_eq!(zeile.gegenkonto.as_deref(), Some("DE11520513735120710131"));
        assert_eq!(zeile.betrag_cent, 5_000);
        assert_eq!(zeile.richtung, Richtung::Ausgang);
        assert_eq!(zeile.empfänger_name, "EDEKA SAGT DANKE");
        assert_eq!(zeile.gläubiger_id.as_deref(), Some("DE98ZZZ09999999999"));
        assert_eq!(
            zeile.buchungstag,
            Kalendertag {
                jahr: 2024,
                monat: 6,
                tag: 13
            }
        );
        assert_eq!(
            zeile.valuta,
            Kalendertag {
                jahr: 2024,
                monat: 6,
                tag: 14
            }
        );
        assert_eq!(zeile.verwendungszweck.as_deref(), Some("Einkauf"));
        assert_eq!(zeile.buchungstext.as_deref(), Some("LASTSCHRIFT"));
    }

    #[test]
    fn aus_aggregat_übernimmt_fehlende_optionalfelder_als_none() {
        let kalender = crate::kalender_bridge::KalenderBridge {
            seam: &DeutscherKalenderSeam,
        };
        let mut rohsatz = gültiger_rohsatz();
        rohsatz.gegenkonto = None;
        rohsatz.verwendungszweck = None;
        rohsatz.buchungstext = None;
        let aggregat = ZeilenAggregat::aus_rohzeile(&rohsatz, &kalender).expect("Pflichtfelder gesetzt");

        let zeile = Zeile::aus_aggregat(&aggregat);

        assert!(zeile.gegenkonto.is_none());
        assert!(zeile.verwendungszweck.is_none());
        assert!(zeile.buchungstext.is_none());
    }
}

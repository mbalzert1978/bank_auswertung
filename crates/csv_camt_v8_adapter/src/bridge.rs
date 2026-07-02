//! Die **Bridges** zu den Nachbar-Slices: sie mappen die rohen Zeilenfelder über
//! die public API von [`obfuskierung_adapter`] und [`eigene_konten_adapter`] auf
//! die Domänen-Werte dieses Slice — **bevor** das [`BuchungAggregat`](csv_camt_v8_domain::BuchungAggregat)
//! gebaut wird. So erreicht kein roher struktureller Identifikator die CSV-Domäne
//! ([ADR-0007](../../../docs/adr/0007-modularer-monolith-feature-slices.md)).

use csv_camt_v8_domain::{
    EmpfängerEntity, GegenkontoEntity, GläubigerIdEntity, KontoIdEntity, ObfuskierterTextEntity,
    PseudonymEntity,
};
use eigene_konten_adapter::{Iban as KontenIban, Kontenregister, Zuordnung};
use obfuskierung_adapter::{Iban as ObfuskierungsIban, Obfuskierer};

use crate::fehler::ImportError;
use crate::rolle::rolle_entity_aus_konten;

/// Obfuskiert das Auftragskonto (eigene IBAN) zur stabilen `KONTO_x`. Prüft zuerst
/// über den Eigene-Konten-Slice, dass es strukturell/prüfsummen-gültig **und** ein
/// eigenes Konto ist (sonst lauter Abbruch), und obfuskiert dann über den
/// Obfuskierungs-Slice die **normalisierte** IBAN — verschiedene Schreibweisen
/// desselben Kontos erhalten so dasselbe Token.
pub(crate) fn obfuskiere_auftragskonto(
    auftragskonto: &str,
    konten: &Kontenregister,
    obfuskierer: &mut Obfuskierer,
) -> Result<KontoIdEntity, ImportError> {
    let iban = KontenIban::new(auftragskonto).map_err(ImportError::AuftragskontoInvalid)?;
    match konten.zuordnung(&iban) {
        Zuordnung::Eigen(_) => {
            let normalisiert =
                ObfuskierungsIban::new(iban.as_str()).expect("normalisierte IBAN ist nicht leer");
            let konto = obfuskierer.obfuskiere_konto(&normalisiert);
            Ok(KontoIdEntity::new(konto.as_str()).expect("KONTO-Token ist nicht leer"))
        }
        Zuordnung::Fremd => Err(ImportError::AuftragskontoUnknown),
    }
}

/// Löst die Gegenseite über den Eigene-Konten-Slice auf: eigene IBAN → `Eigen` (mit
/// auf die slice-eigene Rolle gemappter Rolle), sonst `Fremd`. Eine leere oder
/// nicht als IBAN parsbare Gegenseite ist definitiv keines der eigenen Konten →
/// `Fremd`.
pub(crate) fn gegenkonto(gegen_iban: &str, konten: &Kontenregister) -> GegenkontoEntity {
    match KontenIban::new(gegen_iban) {
        Ok(iban) => match konten.zuordnung(&iban) {
            Zuordnung::Eigen(rolle) => GegenkontoEntity::Eigen(rolle_entity_aus_konten(rolle)),
            Zuordnung::Fremd => GegenkontoEntity::Fremd,
        },
        Err(_) => GegenkontoEntity::Fremd,
    }
}

/// Löst den Empfänger auf. Die Fail-safe-Weiche „Gläubiger-ID → Firma, sonst
/// Person" bleibt Domänenlogik ([`EmpfängerEntity::resolve`]); die Bridge reicht
/// die Rohdaten und eine **lazy**, über den Obfuskierungs-Slice pseudonymisierende
/// Quelle herein (nur im Person-Fall abgefragt, damit Firmen keinen Zähler
/// verbrauchen). Ergibt die Auflösung `None` (weder Gläubiger-ID noch Name), ist
/// die Gegenpartei nicht identifizierbar → `EmpfängerMissing`.
pub(crate) fn empfänger(
    glaeubiger_id: &str,
    beguenstigter: &str,
    obfuskierer: &mut Obfuskierer,
) -> Result<EmpfängerEntity, ImportError> {
    EmpfängerEntity::resolve(GläubigerIdEntity::new(glaeubiger_id), || {
        obfuskierer
            .obfuskiere_person(beguenstigter)
            .map(|pseudonym| {
                PseudonymEntity::new(pseudonym.as_str()).expect("PERSON-Token ist nicht leer")
            })
    })
    .ok_or(ImportError::EmpfängerMissing)
}

/// Führt den Verwendungszweck-Freitext durch den Obfuskierungs-Engpass des
/// Nachbar-Slice. `None`, wenn nach dem Trimmen nichts bleibt (kein leerer Text).
pub(crate) fn verwendungszweck(
    roh: &str,
    obfuskierer: &Obfuskierer,
) -> Option<ObfuskierterTextEntity> {
    obfuskierer.obfuskiere_text(roh).map(|text| {
        ObfuskierterTextEntity::new(text.as_str()).expect("obfuskierter Text ist nicht leer")
    })
}

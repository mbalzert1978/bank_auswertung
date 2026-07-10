/// Die **Identität**, die der Slice einem eingelesenen Aggregat vergibt — der je abgelegter
/// Roh-Zeile eindeutige Schlüssel, unter dem die Roh-Erfassung persistiert und den der Slice als
/// **Zeiger** publiziert (Tabelle = Wahrheit,
/// Message = Pointer,
/// [0017](../../../docs/issues/0017-einlesen-publiziert-nur-die-aggregat-id-statt-des-kompletten-aggregats.md)).
/// Der nachgelagerte `obfuskieren`-Slice lädt das Aggregat später **per dieser ID** aus dem
/// einlesen-Speicher. Opak — **kein** struktureller Identifikator, nie ein Rohwert.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IngestionId(String);

impl IngestionId {
    /// Baut die ID aus ihrem Rohwert (der [`Store`](crate::Store)-Port vergibt sie beim Ablegen und
    /// reicht sie herein). Bewusst **opak** (kein `NonEmptyString`, keine fehlbare Signatur): der
    /// Slice deutet die ID nie, er reicht sie durch — ein eigener Validierungspfad wäre reibungsvolle
    /// Zeremonie ohne Gewinn. Die **Nicht-Leere** ist dennoch im Typ abgesichert: die ID vergibt io
    /// (`INSERT … RETURNING id`), eine leere ID ist ein **unmöglicher** Zustand — ein Leerwert wäre
    /// ein Herkunfts-Invariantenbruch (der Slice publizierte einen Leer-Zeiger, `obfuskieren` liefe
    /// in eine Retry-Endlosschleife). Genau darum fällt der Guard **hart** um (`panic`) statt einen
    /// `Result`-Pfad für einen nie eintretenden Fall aufzuspannen.
    ///
    /// # Panics
    /// Wenn der Rohwert leer ist — der Speicher hat die Herkunfts-Garantie (nie-leere ID) gebrochen.
    pub fn new(value: impl Into<String>) -> Self {
        let value = value.into();
        assert!(
            !value.is_empty(),
            "IngestionId: leere ID — der Speicher (io) garantiert eine nie-leere Identität, \
             ein Leerwert ist ein Invariantenbruch"
        );
        Self(value)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::IngestionId;

    #[test]
    #[should_panic(expected = "leere ID")]
    fn leere_id_faellt_hart_um() {
        // Der Speicher garantiert eine nie-leere Identität — ein Leerwert ist ein
        // Invariantenbruch, kein deutbarer Eingang: der Guard fällt hart um.
        IngestionId::new("");
    }

    #[test]
    fn nicht_leere_id_wird_opak_durchgereicht() {
        let id = IngestionId::new("ingested-1");
        assert_eq!(id.as_str(), "ingested-1");
    }
}

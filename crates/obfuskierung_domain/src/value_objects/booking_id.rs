/// Die **Identität** eines eingelesenen Aggregats — der Schlüssel, unter dem der Slice das
/// obfuskierte Aggregat ablegt und den er als **Zeiger** publiziert (Tabelle = Wahrheit, Message =
/// Pointer). Aktuell die opake Envelope-ID der eingehenden Nachricht; ab
/// [0017](../../../docs/issues/0017-einlesen-publiziert-nur-die-aggregat-id-statt-des-kompletten-aggregats.md)
/// die von `einlesen` vergebene Aggregat-ID, über die der Handler das Aggregat aus dem
/// einlesen-Speicher lädt. Opak — kein struktureller Identifikator.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BookingId(String);

impl BookingId {
    /// Baut die ID aus ihrem Rohwert (der Adapter reicht die Envelope-ID herein). Bewusst **opak**
    /// (kein `NonEmptyString`, keine fehlbare Signatur): der Slice deutet die ID nie, er reicht sie
    /// als Zeiger durch. Die **Nicht-Leere** ist dennoch im Typ abgesichert: die ID vergibt die
    /// Herkunft (io/Outbox-Envelope, nie leer), eine leere ID ist ein **unmöglicher** Zustand — ein
    /// Herkunfts-Invariantenbruch. Genau darum fällt der Guard **hart** um (`panic`) statt einen
    /// `Result`-Pfad für einen nie eintretenden Fall aufzuspannen. (Spiegelt
    /// [`IngestionId::new`](../../../einlesen_domain/src/value_objects/ingestion_id.rs).)
    ///
    /// # Panics
    /// Wenn der Rohwert leer ist — die Herkunft (io/Outbox-Envelope) hat die Garantie (nie-leere ID)
    /// gebrochen.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        let value = value.into();
        assert!(
            !value.is_empty(),
            "BookingId: leere ID — die Herkunft (io/Outbox-Envelope) garantiert eine nie-leere \
             Identität, ein Leerwert ist ein Invariantenbruch"
        );
        Self(value)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

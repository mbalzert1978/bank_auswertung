/// Envelope einer gespeicherten Outbox-Nachricht, wie sie dem Handler übergeben
/// wird: Identität ([`id`](OutboxMessage::id)), Dispatch-Key
/// ([`message_type`](OutboxMessage::message_type)), die Aggregat-Identität
/// ([`partition_key`](OutboxMessage::partition_key), auf der der Verarbeitungszyklus
/// gruppiert), die serialisierte Nutzlast ([`content`](OutboxMessage::content)) und
/// der Ereigniszeitpunkt. **Reines Transport-DTO** — Persistenz-Statusspalten
/// (verarbeitet-Zeitpunkt, Fehler) leben auf der Storage-Zeile, nicht hier.
///
/// `content` folgt der einen JSON-Konvention (s. [`content`](OutboxMessage::content));
/// der Kernel serialisiert selbst **nicht** (serde-frei) — das tun `io` (Publisher)
/// und die Consume-Bridge (Deserialisierung) mit derselben Konvention.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutboxMessage {
    /// Eindeutige Identität **der Nachricht** (UUID-Textform; die Erzeugung liegt in `io`).
    /// **Konvention:** dies ist die Message-UUID, **nicht** der Aggregat-Zeiger — wer auf ein
    /// Aggregat zeigt (z. B. der `Ingested`-ID-Zeiger der 0017-Pipeline), liest den
    /// [`partition_key`](OutboxMessage::partition_key), nie dieses Feld.
    pub id: String,
    /// Dispatch-Key, auf den der Prozessor den passenden
    /// [`OutboxHandler`](crate::OutboxHandler) auflöst — dieselbe Quelle wie
    /// [`OutboxCommand::MESSAGE_TYPE`](crate::OutboxCommand::MESSAGE_TYPE).
    pub message_type: String,
    /// Aggregat-Identität, auf der der Zyklus serialisiert (gleiche Keys der Reihe
    /// nach, verschiedene Keys nebenläufig). **Konvention:** dies ist zugleich der
    /// **Aggregat-Zeiger** — ein ID-Zeiger auf ein Aggregat (etwa der `Ingested`-Zeiger,
    /// den `obfuskieren` im einlesen-Speicher auflöst) reist **hier**, nicht in
    /// [`id`](OutboxMessage::id) (das die Message-UUID trägt).
    pub partition_key: String,
    /// Serialisierte Nutzlast (JSON-Text nach der hier beschriebenen Konvention).
    ///
    #[doc = "**Die eine JSON-Konvention für Outbox-Nutzlasten** — **nur Dokumentation, keine"]
    #[doc = "serde-Impl im Kernel** (der `shared_kernel` bleibt dependency-frei)."]
    #[doc = ""]
    #[doc = "`io` (der `OutboxPublisher`) serialisiert den Kommando-Inhalt mit dieser"]
    #[doc = "Konvention, und **jede** Consume-Bridge deserialisiert ihren Request mit"]
    #[doc = "derselben — so rundläuft die Wire-Form ohne Casing-Drift. Konvention:"]
    #[doc = "**camelCase**-Feldnamen, **case-insensitiv** beim Lesen (Entsprechung zu"]
    #[doc = "`JsonSerializerDefaults.Web` der C#-Referenz `Outbox/Adapter/OutboxPayload.cs`)."]
    #[doc = "Eine Quelle — Serialisierung, Deserialisierung und die Test-Seams können nicht"]
    #[doc = "auf drei unabhängig gewählte Konventionen auseinanderdriften."]
    pub content: String,
    /// Zeitpunkt des auslösenden Ereignisses als **Unix-Zeit** (Millisekunden seit
    /// Epoch, UTC). Bewusst ein `i64` statt `SystemTime`: als SQLite-`INTEGER`
    /// nativ speicherbar und wire-/serde-neutral — der Envelope trägt keine
    /// plattform-spezifische Zeitrepräsentation.
    pub occurred_on_utc: i64,
    /// Bisherige Zustellversuche (für Retry/Dead-Letter-Entscheidungen).
    pub attempt: u32,
}

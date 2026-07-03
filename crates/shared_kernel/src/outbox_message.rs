/// Envelope einer gespeicherten Outbox-Nachricht, wie sie dem Handler übergeben
/// wird: Identität ([`id`](OutboxMessage::id)), Dispatch-Key
/// ([`message_type`](OutboxMessage::message_type)), die Aggregat-Identität
/// ([`partition_key`](OutboxMessage::partition_key), auf der der Verarbeitungszyklus
/// gruppiert), die serialisierte Nutzlast ([`content`](OutboxMessage::content)) und
/// der Ereigniszeitpunkt. **Reines Transport-DTO** — Persistenz-Statusspalten
/// (verarbeitet-Zeitpunkt, Fehler) leben auf der Storage-Zeile, nicht hier.
///
/// `content` folgt der einen JSON-Konvention (s. [`outbox_payload`]); der
/// Kernel serialisiert selbst **nicht** (serde-frei) — das tun `io` (Publisher)
/// und die Consume-Bridge (Deserialisierung) mit derselben Konvention.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutboxMessage {
    /// Eindeutige Identität der Nachricht (UUID-Textform; die Erzeugung liegt in `io`).
    pub id: String,
    /// Dispatch-Key, auf den der Prozessor den passenden
    /// [`OutboxHandler`](crate::OutboxHandler) auflöst — dieselbe Quelle wie
    /// [`OutboxCommand::MESSAGE_TYPE`](crate::OutboxCommand::MESSAGE_TYPE).
    pub message_type: String,
    /// Aggregat-Identität, auf der der Zyklus serialisiert (gleiche Keys der Reihe
    /// nach, verschiedene Keys nebenläufig).
    pub partition_key: String,
    /// Serialisierte Nutzlast (JSON-Text nach [`outbox_payload`]-Konvention).
    pub content: String,
    /// Zeitpunkt des auslösenden Ereignisses als **Unix-Zeit** (Millisekunden seit
    /// Epoch, UTC). Bewusst ein `i64` statt `SystemTime`: als SQLite-`INTEGER`
    /// nativ speicherbar und wire-/serde-neutral — der Envelope trägt keine
    /// plattform-spezifische Zeitrepräsentation.
    pub occurred_on_utc: i64,
    /// Bisherige Zustellversuche (für Retry/Dead-Letter-Entscheidungen).
    pub attempt: u32,
}

/// Die eine JSON-Konvention für Outbox-Nutzlasten — **nur Dokumentation, keine
/// serde-Impl im Kernel** (der `shared_kernel` bleibt dependency-frei).
///
/// `io` (der `OutboxPublisher`) serialisiert den Kommando-Inhalt mit dieser
/// Konvention, und **jede** Consume-Bridge deserialisiert ihren Request mit
/// derselben — so rundläuft die Wire-Form ohne Casing-Drift. Konvention:
/// **camelCase**-Feldnamen, **case-insensitiv** beim Lesen (Entsprechung zu
/// `JsonSerializerDefaults.Web` der C#-Referenz `Outbox/Adapter/OutboxPayload.cs`).
/// Eine Quelle — Serialisierung, Deserialisierung und die Test-Seams können nicht
/// auf drei unabhängig gewählte Konventionen auseinanderdriften.
pub mod outbox_payload {}

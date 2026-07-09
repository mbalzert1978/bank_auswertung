use crate::value_objects::ObfuscatedBooking;

/// Der von der Domäne **verlangte** Port, um das obfuskierte Aggregat in die `obfuskiert`-Tabelle zu
/// **legen**. Wie beim Einlese-`Store` *staged* der Handler nur (reiht in die offene Arbeitseinheit
/// ein) — **wann** committet wird, weiß die Domäne nicht: das ist io-Sache (die Outbox klammert die
/// Ablage **und** das Einreihen der `Obfuscated`-Nachricht in **eine** `UnitOfWork`,
/// [ADR-0008](../../../docs/adr/0008-event-getriebene-pipeline-und-outbox.md)). Darum ist `save`
/// fehlerfrei (`void`).
pub trait ObfuscatedRepository {
    /// Reiht das obfuskierte Aggregat in die offene Arbeitseinheit ein (Staging).
    fn save(&self, booking: &ObfuscatedBooking);
}

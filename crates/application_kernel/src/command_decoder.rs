/// Bildet die extrahierte Outbox-Envelope am **Slice-Eingang** auf das interne Kommando ab
/// (reine ACL). **Shape Y:** der Slice nennt am Eingang **keinen** Outbox-Typ — `io`
/// extrahiert die Envelope und reicht dem Decoder nur die **nackten Strings**
/// `partition_key`/`content`. So bleibt die Outbox-Mechanik für den Slice-Adapter
/// compiler-echt unsichtbar
/// ([ADR-0011](../../../docs/adr/0011-slice-reine-pipeline-outbox-mechanik-in-io.md),
/// Entscheidung C).
///
/// `decode` ist eine **assoziierte Funktion ohne `self`**: der Decoder trägt keinen Zustand,
/// er ist reine Abbildung `(partition_key, content) -> Option<Command>`. `None` signalisiert
/// leeren/ungültigen Inhalt; `io` mappt das auf ein `Critical`-Dead-Letter (Envelope-/
/// Dispatch-Ebene). Ein einlese-artiger Auslöser ohne Nutzlast liefert dagegen stets
/// `Some` — leerer `content` ist dort **gültig**, kein Fehler.
pub trait CommandDecoder {
    /// Das interne Kommando, das dieser Decoder aus der Envelope baut.
    type Command;

    /// Baut das interne Kommando aus den nackten Envelope-Strings; `None` bei
    /// leerem/ungültigem Inhalt (den `io` zu einem `Critical`-Dead-Letter macht).
    fn decode(partition_key: &str, content: &str) -> Option<Self::Command>;
}

//! Das adapter-lokale Wire-DTO des Slice: ausgehend [`IngestedMessage`] (Domäne → Wire, ein
//! reiner **ID-Zeiger** ohne serde — die Nutzlast liegt im Speicher). Es trägt die
//! Dispatch-Kenntnis, die die serde-freie Domäne bewusst nicht kennt. Eingehend gibt es kein
//! Wire-DTO: die `ReadTransactions`-Nachricht ist reiner Auslöser (feldlos, ohne Inhalt), die
//! [`ConsumeBridge`](crate::bridges::ConsumeBridge) baut das Kommando direkt aus dem Envelope.

pub(crate) mod ingested_message;

pub(crate) use ingested_message::IngestedMessage;

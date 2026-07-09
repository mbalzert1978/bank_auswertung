//! Das ausgehende Wire-Command des Slice: [`ObfuscatedMessage`] trägt **nur die ID** der
//! abgelegten Buchung (Zeiger, nicht Nutzlast). Adapter-/io-Sache — die Domäne kennt keine
//! Wire-Form.

mod obfuscated_message;

pub(crate) use obfuscated_message::ObfuscatedMessage;

use crate::commands::{RawLine, RunConfiguration};
use crate::error::SourceError;

/// Der von der Domäne **verlangte** Datenquellen-Port, über den der
/// [`ReadHandler`](crate::ReadHandler) den rohen Batch **beschafft** (statt ihn per Kommando
/// transportiert zu bekommen,
/// [Issue 0018](../../../docs/issues/0018-einlesen-beschafft-zeilen-ueber-datenquellen-port-statt-per-command.md)).
/// Er liefert die Zeilen bereits domänen-typisiert ([`RawLine`]) und ist **fehlbar**: eine
/// misslungene Beschaffung ist ein domänen-typisierter, redigierter [`SourceError`] —
/// **terminal**, nie Abwesenheit (der Adapter mappt ihn auf ein Dead-Letter). **io erfüllt
/// ihn später** über den Quell-Seam des Adapters (die Präsenz-Prüfung liegt hinter dem Seam
/// in der Port-Bridge); in den Specs trägt ihn ein Fake. Ein leerer Batch ist gültig — es
/// passiert nichts.
pub trait RawLineSource {
    /// Beschafft den Batch der Quelle, parametrisiert durch die Konfiguration des Laufs.
    fn fetch(&self, configuration: &RunConfiguration) -> Result<Vec<RawLine>, SourceError>;
}

use super::NonEmptyString;

/// Der **Zeilen-Schlüssel** einer Buchung — die Identität, an der ein erneuter Import dieselbe
/// Zeile wiedererkennt ([ADR-0016](../../../docs/adr/0016-zeilen-schluessel-und-lauf-eigene-fenster.md)).
/// Ein neuer Import **ergänzt**: eine Zeile, deren Schlüssel schon abgelegt ist, wird weder erneut
/// angelegt noch erneut publiziert.
///
/// **Opak.** Wie der Schlüssel entsteht (Auftragskonto + Kundenreferenz, sonst alle Spalten + die
/// laufende Nummer gleicher Zeilen), ist **Format-Wissen** und lebt hinter dem Quell-Seam in `io`;
/// die Domäne vergleicht ihn nur. Sie deutet ihn nie und kennt seine Bestandteile nicht — darum
/// trägt er auch keinen Rohwert, den sie zeigen könnte: `io` reicht einen Digest herein.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowKey(NonEmptyString);

impl From<NonEmptyString> for RowKey {
    fn from(value: NonEmptyString) -> Self {
        Self(value)
    }
}

impl RowKey {
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

use obfuskierung_domain::{KnownPersons, NonEmptyString};

/// Nennt `text` einen der `names` — erkannt **genau so**, wie der Freitext-Engpass dieses Slice einen
/// bekannten Personen-Namen ersetzen würde (ganzes Wort, ohne Rücksicht auf Groß-/Kleinschreibung,
/// auch in der Bank-Schreibweise „Nachname, Vorname").
///
/// **Warum das öffentlich ist.** Der Bericht in `io` (Issue 0012) zeigt einen Firmennamen nur, wenn er
/// **keinen** Namen der lokalen Namensliste nennt — eine natürliche Person mit Gläubiger-ID soll dort
/// nicht mit Klarnamen stehen. Baute `io` die Erkennung nach, könnten die beiden Regeln
/// auseinanderlaufen; so gibt es **eine**. Leere Einträge der Liste zählen nicht.
#[must_use]
pub fn names_known_person(text: &str, names: &[String]) -> bool {
    KnownPersons::new(names.iter().filter_map(NonEmptyString::new)).named_in(text)
}

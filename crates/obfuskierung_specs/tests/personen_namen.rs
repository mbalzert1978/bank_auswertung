//! Black-Box-Spezifikation von `names_known_person` — der Erkennung eines bekannten Personen-Namens,
//! die der Bericht (Issue 0012) für Firmennamen nutzt. Sie muss **dieselbe** sein wie die des
//! Freitext-Engpasses; belegt über die öffentliche Fläche des Adapters. Alle Namen sind erfunden.

use obfuskierung_adapter::names_known_person;

fn liste() -> Vec<String> {
    vec!["Erika Beispiel".to_string(), String::new()]
}

#[test]
fn ein_name_der_liste_wird_in_jeder_schreibweise_des_scrubbings_erkannt() {
    for text in [
        "Erika Beispiel",
        "ERIKA BEISPIEL",
        "Beispiel, Erika",
        "Vermietung Erika Beispiel",
    ] {
        assert!(names_known_person(text, &liste()), "{text}");
    }
}

#[test]
fn ein_firmenname_ohne_listennamen_wird_nicht_erkannt() {
    for text in ["STROMWERK NORD AG", "Erika", "Beispielhaft GmbH", ""] {
        assert!(!names_known_person(text, &liste()), "{text}");
    }
}

#[test]
fn eine_leere_liste_erkennt_nichts() {
    assert!(!names_known_person("Erika Beispiel", &[]));
}

#[test]
fn ein_gescrubbter_text_wird_an_seinem_platzhalter_erkannt() {
    use obfuskierung_adapter::is_scrubbed;

    for text in ["[PERSON]", "PRAXIS [PERSON]", "Miete [IBAN]", "BIC [BIC]"] {
        assert!(is_scrubbed(text), "{text}");
    }
    for text in ["STROMWERK NORD AG", "PERSON", "[PRAXIS]"] {
        assert!(!is_scrubbed(text), "{text}");
    }
}

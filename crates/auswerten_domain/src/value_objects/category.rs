use crate::value_objects::NonEmptyString;

/// Die **Kategorie** — die semantische Sammelschublade einer Buchungs*art* (CONTEXT.md
/// „Kategorie": z. B. „Lebensmittel", „Spiel & Spaß", „Mobilität").
///
/// Sie wird **vergeben, nicht gerechnet**: das Orakel hinter dem [`Categorizer`](crate::Categorizer)
/// liefert sie, die Domäne ordnet sie über den Fingerabdruck zu. Genau deshalb ist sie ein
/// **Etikett** und kein Summentyp — welche Schubladen es gibt, ist keine Aussage der Analyse,
/// sondern eine des Orakels; ein festes `enum` hier machte den Slice bei jeder neuen Schublade
/// änderungsbedürftig und wäre zugleich die Behauptung, die Domäne kenne die Liste.
///
/// Nicht-leer per Konstruktion ([`NonEmptyString`]): eine leere Kategorie ist keine Kategorie,
/// sondern eine ausgebliebene Antwort — und die trägt gar keinen Eintrag.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Category(NonEmptyString);

impl From<NonEmptyString> for Category {
    fn from(label: NonEmptyString) -> Self {
        Self(label)
    }
}

impl Category {
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

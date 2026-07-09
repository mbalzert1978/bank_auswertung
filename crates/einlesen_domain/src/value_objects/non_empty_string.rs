/// Ein getrimmter, garantiert nicht-leerer Text — der eine validierende Kern, aus dem
/// die nicht-leeren Text-Wert-Typen des Slice (`CreditorId`, `RecipientName`, …) gebaut
/// werden, **und** der Typ, den die Domäne an ihrer Grenze anfordert: der Adapter mappt
/// jeden rohen (nicht-leeren) CSV-Wert auf einen `NonEmptyString`, die Domäne parst ihn
/// dann weiter — so spricht die Domäne nie ein primitives `String`/`&str`. Getrimmt und
/// Leeres genau **hier** abgewiesen, sodass kein Wert-Typ die Prüfung wiederholt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NonEmptyString(String);

impl NonEmptyString {
    /// Baut den Kern: getrimmt; leer oder nur Whitespace → `None`.
    pub fn new(raw: &str) -> Option<Self> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(Self(trimmed.to_string()))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::NonEmptyString;

    #[test]
    fn leerer_und_nur_whitespace_text_wird_abgelehnt() {
        assert_eq!(NonEmptyString::new(""), None);
        assert_eq!(NonEmptyString::new("   "), None);
    }

    #[test]
    fn nicht_leerer_text_wird_getrimmt() {
        let kern = NonEmptyString::new("  EDEKA  ").expect("nicht leer");
        assert_eq!(kern.as_str(), "EDEKA");
    }
}

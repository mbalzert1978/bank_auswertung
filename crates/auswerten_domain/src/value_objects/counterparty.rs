use crate::value_objects::{NonEmptyString, Recipient};

/// **Wer auf der anderen Seite stand — und wofür.** Die Gegenpartei einer Buchung
/// ([`Recipient`]) zusammen mit ihrem obfuskierten Verwendungszweck.
///
/// **Warum die beiden einen Typ bilden.** Sie werden nie einzeln gelesen: der
/// [`Fingerprint`](crate::Fingerprint) braucht bei einem Zahlungsintermediär **beide** (die
/// Gläubiger-ID des Durchleiters *und* den Händler-Token aus dem Zweck, sonst verklumpen die
/// dahinterliegenden Händler), und der `category_text` fürs Orakel setzt sich
/// aus **beiden** zusammen. Als zwei lose Felder stand dieses Paar zweimal unbenannt im Code; hier
/// trägt es einen Namen.
///
/// **Der Zweck ist streng genommen keine Eigenschaft der Gegenpartei** — er ist, was bei der Zahlung
/// hingeschrieben wurde. Er steht trotzdem hier, weil er genau dort auswertbar wird: bei einem
/// Durchleiter benennt er den *eigentlichen* Empfänger, den die Gegenpartei verdeckt. Getrennt zu
/// führen, was nur gemeinsam etwas aussagt, wäre die schlechtere Modellierung.
///
/// **Datenschutz:** beides steht stromabwärts der Obfuskierungs-Grenze — Pseudonym oder öffentliche
/// Gläubiger-ID, dazu bereits gescrubbter Freitext. Kein struktureller Identifikator, kein Seed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Counterparty {
    recipient: Recipient,
    /// Echt optional: nicht jede Zeile führt einen Verwendungszweck.
    purpose: Option<NonEmptyString>,
}

impl Counterparty {
    /// Setzt die Gegenpartei-Angabe aus ihren bereits geprüften Teilen zusammen — der Adapter (ACL)
    /// ruft sie beim Deuten „außen → innen".
    #[must_use]
    pub fn new(recipient: Recipient, purpose: Option<NonEmptyString>) -> Self {
        Self { recipient, purpose }
    }

    pub(crate) fn recipient(&self) -> &Recipient {
        &self.recipient
    }

    pub(crate) fn purpose(&self) -> Option<&NonEmptyString> {
        self.purpose.as_ref()
    }

    /// Der **obfuskierte Text**, an dem das Orakel die Kategorie erraten soll: Empfänger **und**
    /// Verwendungszweck, so wie CONTEXT.md („Kategorie") sie nennt — der Firmenname als wichtigstes,
    /// der Freitext als zweitwichtigstes Signal.
    ///
    /// Beide zusammen in **einem** Feld, nicht als zwei: das Orakel bekommt einen Text zum Deuten,
    /// und welche Bestandteile ihn ergeben, ist eine Aussage der Domäne über das Kategorie-Signal —
    /// keine Struktur, die das DTO nach außen tragen müsste.
    ///
    /// Liegt keines von beiden vor, gibt es **nichts zu fragen** (`None`).
    pub(crate) fn category_text(&self) -> Option<NonEmptyString> {
        let signals = [self.recipient.category_signal(), self.purpose()]
            .into_iter()
            .flatten()
            .map(NonEmptyString::as_str)
            .collect::<Vec<_>>();
        NonEmptyString::new(&signals.join(" "))
    }
}

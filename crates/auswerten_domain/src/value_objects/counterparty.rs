use crate::value_objects::party::{Party, egress_words};
use crate::value_objects::{NonEmptyString, Recipient};

/// **Wer auf der anderen Seite stand — und wofür.** Die Gegenpartei einer Buchung
/// ([`Recipient`]) zusammen mit ihrem obfuskierten Verwendungszweck.
///
/// **Warum die beiden einen Typ bilden.** Sie werden nie einzeln gelesen: der
/// [`Fingerprint`](crate::Fingerprint) braucht bei einem Zahlungsintermediär **beide** (die
/// Gläubiger-ID des Durchleiters *und* den Händler-Token aus dem Zweck, sonst verklumpen die
/// dahinterliegenden Händler), und der `category_text` fürs Orakel wählt je nach Art der
/// Gegenpartei eines von beiden. Als zwei lose Felder stand dieses Paar zweimal unbenannt im Code; hier
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

    /// Die Gegenpartei, **einmal** eingeordnet (Durchleiter / Firma / Person / keine) — die eine
    /// Einordnung, auf der Fingerabdruck und Anfrage-Text beide aufsetzen.
    pub(crate) fn party(&self) -> Party<'_> {
        Party::of(&self.recipient, self.purpose.as_ref())
    }

    /// Der **Text**, an dem das Orakel die Kategorie erraten soll — und damit das, was im Chat-Export
    /// die Maschine verlässt (Issue 0011). Ein vollständiges `match` auf die eingeordnete Gegenpartei,
    /// so knapp wie möglich:
    ///
    /// - **Durchleiter:** **nur** der Verwendungszweck. Der Name („PayPal Europe …") sagt über die
    ///   Kategorie nichts; der Händler steht im Zweck (CONTEXT.md „Zahlungsintermediär").
    /// - **Firma:** **nur** der Firmenname — das wichtigste Kategorie-Signal. Der Zweck bleibt
    ///   draußen: dort stehen Kunden- und Vertragsnummern, oft der Name des Kontoinhabers. Ohne
    ///   Namen gibt es **keine** Anfrage, kein Rückfall auf den Zweck.
    /// - **Person:** **keine** Anfrage. Ihr Fingerabdruck ist ein seed-abgeleitetes Pseudonym, und der
    ///   Zweck einer Überweisung an eine Person ist der Freitext mit dem höchsten Risiko. Preis:
    ///   Kartenzahlungen (Händler ohne Gläubiger-ID gelten fail-safe als Person) und Überweisungen an
    ///   Personen bekommen per Chat kein Etikett.
    /// - **Keine Gegenpartei:** keine Anfrage.
    ///
    /// Auf den gewählten Text läuft die **Egress-Wortregel** (jedes Wort mit Ziffer oder `@` fällt).
    /// Bleibt nichts übrig, gibt es nichts zu fragen (`None`).
    pub(crate) fn category_text(&self) -> Option<NonEmptyString> {
        let source = match self.party() {
            Party::Intermediary { purpose, .. } => purpose,
            Party::Company { name, .. } => name,
            Party::Person { .. } | Party::Absent => None,
        }?;
        NonEmptyString::new(&egress_words(source.as_str()).collect::<Vec<_>>().join(" "))
    }
}

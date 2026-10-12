use crate::ports::Obfuscator;
use crate::value_objects::known_persons::KnownPersons;
use crate::value_objects::non_empty_string::NonEmptyString;
use crate::value_objects::raw_counterparty::RawCounterparty;
use crate::value_objects::recipient::Recipient;
use crate::value_objects::replacement::{AccountToken, ObfuscatedText, Pseudonym};
use crate::value_objects::seed::Seed;

/// Die **obfuskierte** Gegenpartei-Sicht — der Ausgang der Domäne: Konten als Token, der
/// Empfänger fail-safe als [`Recipient`] gedeutet, der Freitext durch den [`ObfuscatedText`]-
/// Engpass. Der Adapter setzt sie später mit dem verbatim durchgereichten Pass-Through (Betrag,
/// Datum …) + Seed + ID zur persistierbaren Buchung zusammen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObfuscatedCounterparty {
    ordering_account: AccountToken,
    counter_account: Option<AccountToken>,
    recipient: Recipient,
    purpose: Option<ObfuscatedText>,
    booking_text: Option<ObfuscatedText>,
}

impl ObfuscatedCounterparty {
    /// Obfuskiert eine rohe Gegenpartei seed-basiert über den [`Obfuscator`]-Port — der eine Ort,
    /// an dem die **Fail-safe-Deutung** Firma/Person fällt (siehe [`Recipient`]). Konten
    /// (Auftraggeber **und** Gegenkonto) werden getokenisiert, der Freitext läuft durch den
    /// Scrubbing-Engpass ([`ObfuscatedText`], mit den bekannten Personen-Namen); die Gläubiger-ID
    /// bleibt als Anker Klartext.
    #[must_use]
    pub(crate) fn obfuscate(
        raw: RawCounterparty,
        seed: Seed,
        obfuscator: &dyn Obfuscator,
        persons: &KnownPersons,
    ) -> Self {
        let token = |plaintext: &NonEmptyString| {
            AccountToken::from_replacement(obfuscator.obfuscate(seed, plaintext))
        };
        let text = |plaintext: NonEmptyString| ObfuscatedText::scrub(&plaintext, persons);
        let ordering_account = token(&raw.ordering_account);
        let counter_account = raw.counter_account.map(|account| token(&account));
        let recipient = match (raw.recipient_name, raw.creditor_id) {
            // Gläubiger-ID vorhanden → sichere Firma. Der Name wird **nicht pseudonymisiert** — er ist
            // ein bewusstes Kategorie-Signal (der Firmenname ist ein öffentlicher Analyse-Anker) —,
            // läuft aber wie aller Freitext durch den Scrubbing-Engpass: steht darin ein bekannter
            // Personen-Name oder eine IBAN, wird auch er ersetzt. Die Gläubiger-ID bleibt Anker.
            (name, Some(creditor_id)) => Recipient::Company {
                name: name.map(text),
                creditor_id: creditor_id.into(),
            },
            // Keine Gläubiger-ID, aber ein Name → fail-safe Privatperson: pseudonymisieren.
            (Some(name), None) => Recipient::Person {
                pseudonym: Pseudonym::from_replacement(obfuscator.obfuscate(seed, &name)),
            },
            // Weder Name noch Gläubiger-ID → keine Gegenpartei (Gebührenzeile).
            (None, None) => Recipient::Absent,
        };
        Self {
            ordering_account,
            counter_account,
            recipient,
            purpose: raw.purpose.map(text),
            booking_text: raw.booking_text.map(text),
        }
    }

    #[must_use]
    pub fn ordering_account(&self) -> &AccountToken {
        &self.ordering_account
    }

    #[must_use]
    pub fn counter_account(&self) -> Option<&AccountToken> {
        self.counter_account.as_ref()
    }

    #[must_use]
    pub fn recipient(&self) -> &Recipient {
        &self.recipient
    }

    #[must_use]
    pub fn purpose(&self) -> Option<&ObfuscatedText> {
        self.purpose.as_ref()
    }

    #[must_use]
    pub fn booking_text(&self) -> Option<&ObfuscatedText> {
        self.booking_text.as_ref()
    }
}

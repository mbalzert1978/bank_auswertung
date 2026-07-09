use obfuskierung_domain::{
    AccountToken, ObfuscatedBooking as DomainBooking, ObfuscatedText as DomainText,
    Pseudonym as DomainPseudonym, Recipient as DomainRecipient,
};

/// Obfuskierte Konto-Kennung (`KONTO_…`) — **adapter-eigenes** Ausgangs-DTO (kein Re-Export des
/// Domänen-`AccountToken`): unbedenklich, kein struktureller Identifikator.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AccountId(String);

impl AccountId {
    /// Projiziert das Domänen-Token auf das DTO — crate-intern (Bridge/TestApi).
    pub(crate) fn from_token(token: &AccountToken) -> Self {
        Self(token.as_str().to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Personen-Pseudonym (`PERSON_…`) — adapter-eigenes Ausgangs-DTO.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Pseudonym(String);

impl Pseudonym {
    /// Projiziert das Domänen-Pseudonym auf das DTO — crate-intern (Bridge/TestApi).
    pub(crate) fn from_domain(pseudonym: &DomainPseudonym) -> Self {
        Self(pseudonym.as_str().to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Obfuskierter Freitext (Firmenname/Zweck/Buchungstext) — adapter-eigenes Ausgangs-DTO.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ObfuscatedText(String);

impl ObfuscatedText {
    /// Projiziert den obfuskierten Domänen-Text auf das DTO — crate-intern (Bridge/TestApi).
    pub(crate) fn from_domain(text: &DomainText) -> Self {
        Self(text.as_str().to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Die obfuskierte Gegenpartei — der Summentyp Firma/Person/keine, **adapter-eigen** projiziert aus
/// `obfuskierung_domain::Recipient` (kein Domänentyp überquert die Grenze). Die `creditor_id` einer
/// Firma bleibt als öffentlicher Anker **Klartext**.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Recipient {
    Company {
        name: Option<ObfuscatedText>,
        creditor_id: String,
    },
    Person {
        pseudonym: Pseudonym,
    },
    Absent,
}

impl Recipient {
    /// Projiziert den Domänen-`Recipient` auf das adapter-eigene DTO.
    pub(crate) fn from_domain(recipient: &DomainRecipient) -> Self {
        match recipient {
            DomainRecipient::Company { name, creditor_id } => Recipient::Company {
                name: name.as_ref().map(ObfuscatedText::from_domain),
                creditor_id: creditor_id.as_str().to_string(),
            },
            DomainRecipient::Person { pseudonym } => Recipient::Person {
                pseudonym: Pseudonym::from_domain(pseudonym),
            },
            DomainRecipient::Absent => Recipient::Absent,
        }
    }
}

/// Die **obfuskierte Buchung** — das Ausgangs-DTO des Slice, das der Adapter in die
/// `obfuskiert`-Tabelle ablegt (die Wahrheit für stromabwärts; die Outbox-Nachricht trägt nur die
/// [`id`](ObfuscatedBooking::id) als Zeiger). Sie setzt die obfuskierte Domänen-Sicht mit dem
/// verbatim durchgereichten Pass-Through, dem [`seed`](ObfuscatedBooking::seed) (mitpersistiert →
/// reproduzierbar) und der ID zusammen.
///
/// **Datenschutz-Grenze:** ab hier fließen nur Token — dieses DTO trägt **keine** rohe
/// IBAN/Kontonummer und leitet bewusst **kein** `Serialize`/`Deserialize` ab (compilergeprüfter
/// `assert_not_impl_any!`-Guard in `lib.rs`). Die lokale Persistenz flacht io separat auf eine
/// Zeilen-Wire-Form ab.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObfuscatedBooking {
    pub id: String,
    pub ordering_account: AccountId,
    pub counter_account: Option<AccountId>,
    pub amount_cent: u64,
    pub currency: String,
    pub direction: String,
    pub recipient: Recipient,
    pub booking_day: String,
    pub value_date: String,
    pub purpose: Option<ObfuscatedText>,
    pub booking_text: Option<ObfuscatedText>,
    pub seed: u64,
}

impl ObfuscatedBooking {
    /// Projiziert das obfuskierte **Domänen-Aggregat** auf das DTO — die einzige Stelle, an der es
    /// die Adapter-Grenze überquert (die Repository-Bridge ruft sie vor dem Ablegen). Token,
    /// Empfänger, durchgereichte Felder, Seed und ID in einem Zug.
    pub(crate) fn from_domain(booking: &DomainBooking) -> Self {
        let counterparty = booking.counterparty();
        let pass_through = booking.pass_through();
        Self {
            id: booking.id().as_str().to_string(),
            ordering_account: AccountId::from_token(counterparty.ordering_account()),
            counter_account: counterparty.counter_account().map(AccountId::from_token),
            amount_cent: pass_through.amount_cent,
            currency: pass_through.currency.as_str().to_string(),
            direction: pass_through.direction.as_str().to_string(),
            recipient: Recipient::from_domain(counterparty.recipient()),
            booking_day: pass_through.booking_day.as_str().to_string(),
            value_date: pass_through.value_date.as_str().to_string(),
            purpose: counterparty.purpose().map(ObfuscatedText::from_domain),
            booking_text: counterparty.booking_text().map(ObfuscatedText::from_domain),
            seed: booking.seed().value(),
        }
    }
}

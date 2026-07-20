use auswerten_domain::{
    AccountToken, Amount, AnalysisWindow, Currency, DayStamp, Direction, NonEmptyString, Recipient,
    SeedMarker, WindowedBooking,
};

/// Das **eingefrorene Auswertungsfenster**, wie der Lese-Seam es zu einer ID liefert: die eine
/// Währung des Fensters und die Buchungen, auf die es per Fremdschlüssel zeigt (io setzt es aus der
/// Fenster-Zeile und ihrem Join auf die `obfuskiert`-Ablage zusammen).
///
/// Reines Zwischen-DTO zwischen dem Seam (io) und der Domäne; die Abbildung „außen → innen" lebt
/// hier (ACL). **Kein serde**: io baut die Zeile aus dem Speicher, sie wird nicht aus einer
/// Nachricht deserialisiert (die Nachricht trägt nur die ID).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AnalysisWindowRow {
    pub currency: String,
    /// Das **opake Kennzeichen der Tokenisierung**, unter der die Token dieses Fensters entstanden
    /// sind — **nicht** der Seed. io reicht es mit, damit die Eigene-Konten-Config dagegen geprüft
    /// werden kann (ADR-0010: der Seed selbst erreicht diesen Slice nie).
    pub seed_marker: String,
    pub bookings: Vec<WindowBookingRow>,
}

/// Eine **Buchung des Fensters** als rohe Zeile — die drei Zellen, die der Cashflow verwertet (der
/// Buchungstag, der den Monats-Bucket bestimmt; der **vorzeichenlose** Betrag in Cent; die Richtung,
/// in der das Vorzeichen lebt, ADR-0005), plus die zwei, aus denen der **Fingerabdruck** entsteht:
/// die obfuskierte Gegenpartei und der obfuskierte Verwendungszweck.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WindowBookingRow {
    /// Der Buchungstag als sortierbarer Tag-Stamp (ADR-0013, Entscheidung C).
    pub booking_day: i64,
    pub amount_cent: u64,
    pub direction: BookingDirection,
    /// Die Gegenpartei, wie `obfuskieren` sie abgelegt hat (Firma/Person/keine).
    pub recipient: BookingRecipient,
    /// Der obfuskierte Verwendungszweck — leer/fehlend ist zulässig (nicht jede Zeile führt einen).
    pub purpose: Option<String>,
    /// Das **Gegenkonto als Token** (`KONTO_…`), wie `obfuskieren` es abgelegt hat — **nie** eine
    /// IBAN. Echt optional: eine Entgeltzeile hat keine Gegenseite.
    pub counter_account: Option<String>,
}

/// Die **Gegenpartei** als adapter-eigener DTO-Typ. Wie [`BookingDirection`] bewusst
/// **re-deklariert** statt aus der Domäne re-exportiert: kein Domänen-Internum überquert die Grenze.
/// Er spiegelt den Summentyp, den `obfuskieren` an der Datenschutz-Grenze gebildet hat — eine Person
/// trägt **nur** ihr Pseudonym, eine Firma **nie** eines.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum BookingRecipient {
    /// Sichere Firma: Klartext-Name (falls geführt) + die Gläubiger-ID als öffentlicher Anker.
    Company {
        name: Option<String>,
        creditor_id: String,
    },
    /// Fail-safe Privatperson: **nur** das seed-basierte `PERSON_…`-Pseudonym.
    Person { pseudonym: String },
    /// Keine Gegenpartei — z. B. eine Entgeltzeile.
    #[default]
    Absent,
}

/// Die Richtung als **adapter-eigener DTO-Typ**. Bewusst **re-deklariert** statt aus der Domäne
/// re-exportiert: kein Domänen-Internum überquert die Grenze (dieselbe Linie, die `einlesen` bei
/// `Direction`/`Currency` zieht).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum BookingDirection {
    Incoming,
    #[default]
    Outgoing,
}

impl AnalysisWindowRow {
    /// Deutet die rohe Zeile in das Domänen-Aggregat [`AnalysisWindow`] (ACL, „außen → innen").
    /// **Hier erzwingt der Adapter die Präsenz** der Pflichtfelder: fehlt/ist die Währung leer, wird
    /// das zu `Err(MissingMandatoryField)` mit **nur** dem Feldnamen — den die
    /// [`SourceBridge`](crate::adapter::SourceBridge) auf den terminalen
    /// `EvaluationError::MissingMandatoryField` mappt (redigiert, statischer Feldname).
    ///
    /// Buchungstag, Betrag und Richtung brauchen **keine** Präsenz-Prüfung: sie sind schon im Typ
    /// vollständig (ADR-0005 — illegale Zustände unkonstruierbar, nicht „geprüft").
    pub(crate) fn to_window(&self) -> Result<AnalysisWindow, MissingMandatoryField> {
        let currency = Currency::from(mandatory(&self.currency, "currency")?);
        let seed_marker = SeedMarker::from(mandatory(&self.seed_marker, "seed_marker")?);
        Ok(AnalysisWindow::rehydrate(
            currency,
            seed_marker,
            self.bookings
                .iter()
                .map(WindowBookingRow::to_booking)
                .collect::<Result<Vec<_>, _>>()?,
        ))
    }
}

impl WindowBookingRow {
    /// Deutet die rohe Buchungs-Zeile in die Domänen-Sicht. Buchungstag, Betrag und Richtung sind
    /// schon im Typ vollständig; **fehlbar** ist die Zeile allein über ihre Gegenpartei, deren
    /// Schlüssel-Zelle der Adapter hier auf Präsenz prüft (ACL).
    pub(crate) fn to_booking(&self) -> Result<WindowedBooking, MissingMandatoryField> {
        Ok(WindowedBooking::hydrate(
            DayStamp::new(self.booking_day),
            Amount::from_cent(self.amount_cent),
            match self.direction {
                BookingDirection::Incoming => Direction::Incoming,
                BookingDirection::Outgoing => Direction::Outgoing,
            },
            self.recipient.to_recipient()?,
            // Echt optional: ein leerer Zweck ist ein fehlender Zweck, kein Fehler.
            self.purpose.as_deref().and_then(NonEmptyString::new),
            // Ebenso echt optional — und **kein** Präsenz-Pflichtfeld: „keine Gegenseite" ist eine
            // gültige Aussage über eine Entgeltzeile, nicht eine fehlende Angabe.
            self.counter_account
                .as_deref()
                .and_then(NonEmptyString::new)
                .map(AccountToken::from),
        ))
    }
}

impl BookingRecipient {
    /// Deutet die Gegenpartei „außen → innen". **Präsenz-Prüfung (ACL):** trägt die Variante ihre
    /// Schlüssel-Zelle nicht, ist das terminal — eine Firma **ohne** Gläubiger-ID oder eine Person
    /// **ohne** Pseudonym hätte keinen Schlüssel, über den ihr Fingerabdruck stabil bliebe, und
    /// stillschweigend auf einen leeren Schlüssel auszuweichen verklumpte alle solchen Buchungen zu
    /// einer Art. Der Firmen**name** ist dagegen echt optional (er ist Signal, nicht Schlüssel).
    fn to_recipient(&self) -> Result<Recipient, MissingMandatoryField> {
        match self {
            BookingRecipient::Company { name, creditor_id } => Ok(Recipient::Company {
                name: name.as_deref().and_then(NonEmptyString::new),
                creditor_id: mandatory(creditor_id, "creditor_id")?,
            }),
            BookingRecipient::Person { pseudonym } => Ok(Recipient::Person {
                pseudonym: mandatory(pseudonym, "pseudonym")?,
            }),
            BookingRecipient::Absent => Ok(Recipient::Absent),
        }
    }
}

/// Ein Pflichtfeld fehlt (leer/nicht vorhanden) im gelesenen Fenster — **terminaler** Ausgang der
/// „außen → innen"-Abbildung. **Redaktions-Invariante:** trägt **nur** den statischen Feldnamen, nie
/// den Rohwert — auch `Debug` spiegelt so keinen Inhalt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MissingMandatoryField {
    field: &'static str,
}

impl MissingMandatoryField {
    /// Der (statische, unsensible) Name des fehlenden Pflichtfelds — für den redigierten terminalen
    /// Fehlergrund.
    pub(crate) fn field(&self) -> &'static str {
        self.field
    }
}

/// Erzwingt ein Pflichtfeld: reicht die Zelle als `NonEmptyString` an die Domänengrenze, oder —
/// fehlt/leer sie — den redigierten [`MissingMandatoryField`] mit nur dem Feldnamen.
fn mandatory(raw: &str, field: &'static str) -> Result<NonEmptyString, MissingMandatoryField> {
    NonEmptyString::new(raw).ok_or(MissingMandatoryField { field })
}

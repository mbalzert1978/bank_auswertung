use crate::error::LineError;
use crate::ports::{Calendar, Currencies, Money};
use crate::value_objects::{
    Amount, BookingText, CalendarDay, CreditorId, Date, Direction, Iban, NonEmptyString, Purpose,
    RawLine, Recipient, RecipientName, RowKey,
};

/// Eine eingelesene, validierte Buchungszeile — das Aggregat des Slice. Es entsteht
/// **ausschließlich** über `from_raw_line`, das die Rohdaten (der [`RawLine`]-VO) an *einem*
/// Ort parst und die Pflicht/Optional-Entscheidung je Feld trifft. Danach trägt es nur
/// noch gültige Wert-Objekte; rohe Strings gibt es hier nicht mehr. Die Getter liest der
/// Adapter aus, um daraus das ausgehende „Eingelesen"-Command zu bauen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineAggregate {
    row_key: RowKey,
    ordering_account: Iban,
    counter_account: Option<Iban>,
    amount: Amount,
    direction: Direction,
    recipient: Recipient,
    date: Date,
    purpose: Option<Purpose>,
    booking_text: Option<BookingText>,
}

impl LineAggregate {
    /// Deutet eine Rohzeile zu einem Aggregat — die **einzige** Konstruktion. Zuerst die
    /// Pflichtwerte (Währung, Betrag/Richtung aus demselben Betragsfeld über den
    /// `Money`-Port, Datum aus Buchungstag + Valuta über den Kalender, Auftragskonto,
    /// Empfänger), dann die optionalen (Gegenkonto, Verwendungszweck, Buchungstext).
    /// Jeder Fehler ist terminal.
    pub(crate) fn from_raw_line(
        line: RawLine,
        calendar: &dyn Calendar,
        money: &dyn Money,
        currencies: &dyn Currencies,
    ) -> Result<Self, LineError> {
        // Die Pflichtwerte sind **typgarantiert vorhanden** (`RawLine` trägt sie non-`Option`)
        // — kein Präsenz-Check mehr, nur noch das **Deuten**: der Wert liegt vor, kann aber
        // undeutbar sein. Zuerst die per Referenz gelesenen (Währung, Betrag/Richtung,
        // Buchungstag, Valuta), **bevor** die Move-Felder unten aus `line` bewegt werden,
        // sonst kollidiert der Borrow mit dem Teil-Move.
        // „Kennt die Domäne diese Währung?" ist eine Deutungsentscheidung des `Currencies`-Ports
        // (io/ISO 4217): ein unbekannter Code verlässt ihn als `None` → terminaler `CurrencyUnknown`.
        // Die gedeutete Währung trägt **ihre Minor Unit** mit, nach der der `Money`-Port skaliert.
        let currency = currencies
            .resolve(&line.currency)
            .ok_or(LineError::CurrencyUnknown)?;
        // Der Port trägt nur „kein gültiger Betrag" (inkl. fehlendem Vorzeichen oder einem für
        // die Währung zu genauen Wert) als `None` — die terminale `AmountInvalid`-Meldung
        // entsteht hier; das fertig dekodierte, auf die Minor Units **dieser** Währung skalierte
        // VO deutet der Konstruktor unfehlbar (Vorzeichen → Richtung).
        let decoded = money
            .decode(&line.amount, &currency)
            .ok_or(LineError::AmountInvalid)?;
        let (amount, direction) = Amount::with_direction(decoded, currency);
        let booking_day = parse_date(calendar, &line.booking_day, LineError::BookingDayInvalid)?;
        let value_date = parse_date(calendar, &line.value_date, LineError::ValueDateInvalid)?;
        let date = Date::from_days(booking_day, value_date);
        let ordering_account = Iban::from(line.ordering_account);
        let counter_account = line.counter_account.map(Iban::from);
        let recipient = Recipient::from_parts(
            line.recipient_name.map(RecipientName::from),
            line.creditor_id.map(CreditorId::from),
        );
        let purpose = line.purpose.map(Purpose::from);
        let booking_text = line.booking_text.map(BookingText::from);
        Ok(Self {
            row_key: RowKey::from(line.row_key),
            ordering_account,
            counter_account,
            amount,
            direction,
            recipient,
            date,
            purpose,
            booking_text,
        })
    }

    /// Der Zeilen-Schlüssel, an dem ein erneuter Import diese Zeile wiedererkennt.
    pub fn row_key(&self) -> &RowKey {
        &self.row_key
    }

    pub fn ordering_account(&self) -> &Iban {
        &self.ordering_account
    }

    pub fn counter_account(&self) -> Option<&Iban> {
        self.counter_account.as_ref()
    }

    pub fn amount(&self) -> &Amount {
        &self.amount
    }

    pub fn direction(&self) -> Direction {
        self.direction
    }

    pub fn recipient(&self) -> &Recipient {
        &self.recipient
    }

    pub fn date(&self) -> Date {
        self.date
    }

    pub fn purpose(&self) -> Option<&Purpose> {
        self.purpose.as_ref()
    }

    pub fn booking_text(&self) -> Option<&BookingText> {
        self.booking_text.as_ref()
    }
}

/// Deutet ein (typgarantiert vorhandenes) rohes Datumsfeld über den `Calendar`-Port und
/// mappt ein unlesbares Datum auf den terminalen `*Invalid`-Fehler des jeweiligen Felds. Der
/// Port selbst trägt nur `Option` — die Slice-Fehlbedeutung entsteht hier, wo der rohe Wert
/// noch vorliegt.
fn parse_date(
    calendar: &dyn Calendar,
    raw: &NonEmptyString,
    invalid: LineError,
) -> Result<CalendarDay, LineError> {
    calendar.parse(raw).ok_or(invalid)
}

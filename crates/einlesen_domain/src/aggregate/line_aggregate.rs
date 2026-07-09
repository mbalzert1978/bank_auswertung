use crate::commands::RawLine;
use crate::error::LineError;
use crate::ports::Calendar;
use crate::value_objects::{
    Amount, BookingText, CalendarDay, CreditorId, Currency, Date, Direction, Iban, NonEmptyString,
    Purpose, Recipient, RecipientName,
};

/// Eine eingelesene, validierte Buchungszeile — das Aggregat des Slice. Es entsteht
/// **ausschließlich** über `from_raw_line`, das die Rohdaten (der [`RawLine`]-VO) an *einem*
/// Ort parst und die Pflicht/Optional-Entscheidung je Feld trifft. Danach trägt es nur
/// noch gültige Wert-Objekte; rohe Strings gibt es hier nicht mehr. Die Getter liest der
/// Adapter aus, um daraus das ausgehende „Eingelesen"-Command zu bauen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineAggregate {
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
    /// Pflichtwerte (Währung, Betrag/Richtung aus demselben Betragsfeld, Datum aus
    /// Buchungstag + Valuta über den Kalender, Auftragskonto, Empfänger), dann die
    /// optionalen (Gegenkonto, Verwendungszweck, Buchungstext). Jeder Fehler ist terminal.
    pub fn from_raw_line(line: RawLine, calendar: &dyn Calendar) -> Result<Self, LineError> {
        // Zuerst die per Referenz gelesenen Pflichtwerte deuten (Währung, Betrag/Richtung,
        // Buchungstag, Valuta) — **bevor** die Move-Felder unten aus `line` bewegt werden,
        // sonst kollidiert der Borrow mit dem Teil-Move.
        let currency =
            Currency::from_code(line.currency.as_ref().ok_or(LineError::CurrencyMissing)?)?;
        let raw_amount = line.amount.as_ref().ok_or(LineError::AmountMissing)?;
        let (amount, direction) = Amount::with_direction(raw_amount, currency)?;
        let booking_day = parse_date(
            calendar,
            line.booking_day.as_ref(),
            LineError::BookingDayMissing,
            LineError::BookingDayInvalid,
        )?;
        let value_date = parse_date(
            calendar,
            line.value_date.as_ref(),
            LineError::ValueDateMissing,
            LineError::ValueDateInvalid,
        )?;
        let date = Date::from_days(booking_day, value_date);
        let ordering_account = line
            .ordering_account
            .map(Iban::from)
            .ok_or(LineError::OrderingAccountMissing)?;
        let counter_account = line.counter_account.map(Iban::from);
        let recipient = Recipient::from_parts(
            line.recipient_name.map(RecipientName::from),
            line.creditor_id.map(CreditorId::from),
        )?;
        let purpose = line.purpose.map(Purpose::from);
        let booking_text = line.booking_text.map(BookingText::from);
        Ok(Self {
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

    pub fn ordering_account(&self) -> &Iban {
        &self.ordering_account
    }

    pub fn counter_account(&self) -> Option<&Iban> {
        self.counter_account.as_ref()
    }

    pub fn amount(&self) -> Amount {
        self.amount
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

/// Deutet ein rohes Datumsfeld über den `Calendar`-Port und mappt an *einem* Ort auf die
/// terminalen Datumsfehler: ein fehlendes Feld → `*Missing`, ein unlesbares Datum →
/// `*Invalid`. Der Port selbst trägt nur `Option` — die Slice-Fehlbedeutung entsteht
/// hier, wo der rohe Wert noch vorliegt.
fn parse_date(
    calendar: &dyn Calendar,
    raw: Option<&NonEmptyString>,
    missing: LineError,
    invalid: LineError,
) -> Result<CalendarDay, LineError> {
    let raw = raw.ok_or(missing)?;
    calendar.parse(raw).ok_or(invalid)
}

#[cfg(test)]
mod tests {
    use super::LineAggregate;
    use crate::commands::RawLine;
    use crate::error::LineError;
    use crate::ports::Calendar;
    use crate::value_objects::{
        BookingText, CalendarDay, CreditorId, Direction, Iban, NonEmptyString, Purpose,
    };

    fn text(wert: &str) -> Option<NonEmptyString> {
        NonEmptyString::new(wert)
    }

    /// Eine vollständig gültige Rohzeile; ein Test überschreibt einzelne Felder auf
    /// `None`, um genau deren Pflicht-/Optional-Verhalten zu isolieren.
    fn gültige_rohzeile() -> RawLine {
        RawLine {
            ordering_account: text("DE89370400440532013000"),
            booking_day: text("13.06.2024"),
            value_date: text("14.06.2024"),
            amount: text("-50,00"),
            currency: text("EUR"),
            recipient_name: text("EDEKA SAGT DANKE"),
            creditor_id: text("DE98ZZZ09999999999"),
            counter_account: text("DE11520513735120710131"),
            purpose: text("Einkauf"),
            booking_text: text("LASTSCHRIFT"),
        }
    }

    /// Deterministischer Kalender: deutet „TT.MM.JJJJ", sodass Buchungstag und Valuta
    /// im Aggregat unterscheidbar bleiben. Ungültiges Format wird abgewiesen.
    struct DeutscherKalender;

    impl Calendar for DeutscherKalender {
        fn parse(&self, raw: &NonEmptyString) -> Option<CalendarDay> {
            let mut teile = raw.as_str().split('.');
            let tag: u8 = teile.next()?.parse().ok()?;
            let monat: u8 = teile.next()?.parse().ok()?;
            let jahr: u16 = teile.next()?.parse().ok()?;
            CalendarDay::from_parts(jahr, monat, tag)
        }
    }

    fn deute(zeile: RawLine) -> Result<LineAggregate, LineError> {
        LineAggregate::from_raw_line(zeile, &DeutscherKalender)
    }

    #[test]
    fn alle_felder_gesetzt_ergeben_ein_aggregat_mit_korrekten_gettern() {
        let aggregat = deute(gültige_rohzeile()).expect("gültige Zeile");
        assert_eq!(
            aggregat.ordering_account().as_str(),
            "DE89370400440532013000"
        );
        assert_eq!(
            aggregat.counter_account().map(Iban::as_str),
            Some("DE11520513735120710131")
        );
        assert_eq!(aggregat.amount().cent(), 5_000);
        assert_eq!(aggregat.direction(), Direction::Outgoing);
        assert_eq!(aggregat.recipient().name().as_str(), "EDEKA SAGT DANKE");
        assert_eq!(
            aggregat.recipient().creditor_id().map(CreditorId::as_str),
            Some("DE98ZZZ09999999999")
        );
        assert_eq!(
            aggregat.date().booking_day(),
            CalendarDay::from_parts(2024, 6, 13).expect("gültig")
        );
        assert_eq!(
            aggregat.date().value_date(),
            CalendarDay::from_parts(2024, 6, 14).expect("gültig")
        );
        assert_eq!(aggregat.purpose().map(Purpose::as_str), Some("Einkauf"));
        assert_eq!(
            aggregat.booking_text().map(BookingText::as_str),
            Some("LASTSCHRIFT")
        );
    }

    #[test]
    fn fehlende_währung_wird_terminal_abgelehnt() {
        let mut zeile = gültige_rohzeile();
        zeile.currency = None;
        assert_eq!(deute(zeile), Err(LineError::CurrencyMissing));
    }

    #[test]
    fn fehlender_betrag_wird_terminal_abgelehnt() {
        let mut zeile = gültige_rohzeile();
        zeile.amount = None;
        assert_eq!(deute(zeile), Err(LineError::AmountMissing));
    }

    #[test]
    fn fehlendes_auftragskonto_wird_terminal_abgelehnt() {
        let mut zeile = gültige_rohzeile();
        zeile.ordering_account = None;
        assert_eq!(deute(zeile), Err(LineError::OrderingAccountMissing));
    }

    #[test]
    fn fehlender_empfängername_wird_terminal_abgelehnt() {
        let mut zeile = gültige_rohzeile();
        zeile.recipient_name = None;
        assert_eq!(deute(zeile), Err(LineError::RecipientMissing));
    }

    #[test]
    fn fehlender_buchungstag_wird_terminal_abgelehnt() {
        let mut zeile = gültige_rohzeile();
        zeile.booking_day = None;
        assert_eq!(deute(zeile), Err(LineError::BookingDayMissing));
    }

    #[test]
    fn fehlende_valuta_wird_terminal_abgelehnt() {
        let mut zeile = gültige_rohzeile();
        zeile.value_date = None;
        assert_eq!(deute(zeile), Err(LineError::ValueDateMissing));
    }

    #[test]
    fn fehlende_optionalfelder_ergeben_leere_getter() {
        let mut zeile = gültige_rohzeile();
        zeile.counter_account = None;
        zeile.purpose = None;
        zeile.booking_text = None;
        let aggregat = deute(zeile).expect("Pflichtfelder gesetzt");
        assert!(aggregat.counter_account().is_none());
        assert!(aggregat.purpose().is_none());
        assert!(aggregat.booking_text().is_none());
    }

    #[test]
    fn vorzeichenloser_betrag_wird_wegen_fehlendem_vorzeichen_verworfen() {
        // „50,00" ist als Betrag wohlgeformt, aber ohne Vorzeichen ist die Richtung
        // nicht deutbar — die Zeile wird terminal verworfen (nicht als AmountInvalid).
        let mut zeile = gültige_rohzeile();
        zeile.amount = text("50,00");
        assert_eq!(deute(zeile), Err(LineError::SignMissing));
    }

    #[test]
    fn fehler_display_spiegelt_den_rohwert_nicht() {
        // Bei Spaltenverschub landet z. B. eine IBAN im Betragsfeld; die Fehlermeldung
        // darf den Rohwert nicht spiegeln (er könnte in einem persistierten Dead-Letter
        // landen).
        let leck = "DE89370400440532013000";
        let mut zeile = gültige_rohzeile();
        zeile.amount = text(&format!("-{leck}"));
        let fehler = deute(zeile).expect_err("ungültiger Betrag");
        assert!(
            !format!("{fehler}").contains(leck),
            "Display leakt den Rohwert: {fehler}"
        );
    }
}

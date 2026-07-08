/// Ein einzelner Kalendertag als reine Wert-Bestandteile (Jahr/Monat/Tag) —
/// **dependency-frei** getragen (die Domäne bindet kein `chrono`). Die *feine*
/// Kalender-Gültigkeit (dass es keinen 30. Februar gibt) garantiert der `Calendar`-Port,
/// hinter dem in `io` `chrono` steckt; dieses Wert-Objekt prüft im Konstruktor nur die
/// groben Feld-Bereiche (Monat `1..=12`, Tag `1..=31`) und trägt danach den Beweis.
/// Jahr/Monat/Tag geben Zahlen heraus, weil die spätere Rhythmus-Erkennung (`auswerten`)
/// auf Monat und Jahr rechnet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CalendarDay {
    year: u16,
    month: u8,
    day: u8,
}

impl CalendarDay {
    /// Baut einen Kalendertag aus seinen Bestandteilen — der Konstruktor, den der
    /// `Calendar`-Port (in `io`, mit `chrono`) nach dem Deuten aufruft. Grobe
    /// Bereichsprüfung; ein unmöglicher Monat/Tag ergibt **kein** Datum (`None`). Die
    /// terminale `BookingDayInvalid`/`ValueDateInvalid`-Meldung erzeugt der Aufrufer (das
    /// Aggregat), nicht dieser Konstruktor — so trägt er keinen Slice-Fehler.
    pub fn from_parts(year: u16, month: u8, day: u8) -> Option<Self> {
        if (1..=12).contains(&month) && (1..=31).contains(&day) {
            Some(Self { year, month, day })
        } else {
            None
        }
    }

    pub fn year(&self) -> u16 {
        self.year
    }

    pub fn month(&self) -> u8 {
        self.month
    }

    pub fn day(&self) -> u8 {
        self.day
    }
}

/// Die zeitliche Verortung einer Buchung — sie trägt **zwei** Kalendertage: den
/// `booking_day` (wann die Bank gebucht hat) und die `value_date` (Wertstellung/Valuta).
/// Beide beschreiben das „Wann" der Buchung und gehören zusammen, deshalb ein gemeinsames
/// Wert-Objekt statt zweier loser Felder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Date {
    booking_day: CalendarDay,
    value_date: CalendarDay,
}

impl Date {
    /// Bündelt Buchungstag und Valuta zum Datum der Buchung. Beide sind bereits gültige
    /// Kalendertage — hier gibt es nichts mehr abzuweisen.
    pub fn from_days(booking_day: CalendarDay, value_date: CalendarDay) -> Self {
        Self {
            booking_day,
            value_date,
        }
    }

    pub fn booking_day(&self) -> CalendarDay {
        self.booking_day
    }

    pub fn value_date(&self) -> CalendarDay {
        self.value_date
    }
}

#[cfg(test)]
mod tests {
    use super::CalendarDay;

    #[test]
    fn gültige_bestandteile_ergeben_einen_kalendertag() {
        let tag = CalendarDay::from_parts(2024, 6, 13).expect("gültig");
        assert_eq!((tag.year(), tag.month(), tag.day()), (2024, 6, 13));
    }

    #[test]
    fn monat_ausserhalb_eins_bis_zwölf_ergibt_kein_datum() {
        assert!(CalendarDay::from_parts(2024, 13, 1).is_none());
    }

    #[test]
    fn tag_null_ergibt_kein_datum() {
        assert!(CalendarDay::from_parts(2024, 6, 0).is_none());
    }
}

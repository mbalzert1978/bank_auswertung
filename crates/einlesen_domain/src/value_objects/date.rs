/// Ein einzelner Kalendertag als reine Wert-Bestandteile (Jahr/Monat/Tag) —
/// **dependency-frei** getragen (die Domäne bindet kein `chrono`). Die **gesamte**
/// Kalender-Gültigkeit — Feld-Bereiche (Monat `1..=12`, Tag `1..=31`) **wie** die feine
/// Regel (keinen 30. Februar) — garantiert der `Calendar`-Port, hinter dem in `io`
/// `chrono` steckt; ein ungültiges Datum hat den Port schon als `None` verlassen. Dieses
/// Wert-Objekt bündelt danach nur die geprüften Bestandteile. Jahr/Monat/Tag geben Zahlen
/// heraus, weil die spätere Rhythmus-Erkennung (`auswerten`) auf Monat und Jahr rechnet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CalendarDay {
    year: u16,
    month: u8,
    day: u8,
}

impl CalendarDay {
    /// Bündelt die Bestandteile eines **bereits gültigen** Kalendertags — der Konstruktor,
    /// den der `Calendar`-Port (in `io`, mit `chrono`) nach dem erfolgreichen Deuten aufruft.
    /// **Unfehlbar:** die Gültigkeit (inkl. Bereich) prüft der Port, ein ungültiges Datum ist
    /// dort schon `None` geworden; hier gibt es nichts mehr abzuweisen.
    pub fn from_parts(year: u16, month: u8, day: u8) -> Self {
        Self { year, month, day }
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

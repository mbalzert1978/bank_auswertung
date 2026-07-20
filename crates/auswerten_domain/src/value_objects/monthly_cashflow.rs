use crate::value_objects::{Amount, Direction, Month};

/// Der **Cashflow eines Monats** — ein Bucket der ersten periodischen Kennzahl, die dieser Slice
/// **innerhalb** eines Auswertungsfensters rechnet.
///
/// Zu- und Abfluss stehen **getrennt und je vorzeichenlos** nebeneinander, statt zu einem
/// vorzeichenbehafteten Saldo zusammengerechnet zu werden: das Vorzeichen lebt in der
/// [`Direction`](crate::Direction) (ADR-0005), und der Saldo ist daraus jederzeit ableitbar — die
/// Gegenrichtung wäre es nicht. Fürs Aufspüren von Kostentreibern zählt der **Abfluss**, nicht der
/// Saldo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MonthlyCashflow {
    month: Month,
    incoming: Amount,
    outgoing: Amount,
}

impl MonthlyCashflow {
    /// Ein leerer Bucket für einen Monat — der Startwert, in den die Buchungen dieses Monats
    /// hineinfließen.
    #[must_use]
    pub(crate) fn empty(month: Month) -> Self {
        Self {
            month,
            incoming: Amount::default(),
            outgoing: Amount::default(),
        }
    }

    /// Nimmt einen Betrag in **seiner** Richtung auf — die eine Stelle, an der die Richtung
    /// entscheidet, welche der beiden Spalten wächst. Rein: gibt den gewachsenen Bucket zurück,
    /// statt sich zu verändern.
    #[must_use]
    pub(crate) fn plus(self, amount: Amount, direction: Direction) -> Self {
        match direction {
            Direction::Incoming => Self {
                incoming: self.incoming.plus(amount),
                ..self
            },
            Direction::Outgoing => Self {
                outgoing: self.outgoing.plus(amount),
                ..self
            },
        }
    }

    #[must_use]
    pub fn month(&self) -> Month {
        self.month
    }

    #[must_use]
    pub fn incoming(&self) -> Amount {
        self.incoming
    }

    #[must_use]
    pub fn outgoing(&self) -> Amount {
        self.outgoing
    }
}

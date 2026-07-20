use crate::value_objects::{Amount, DayStamp, Direction, Fingerprint};

/// Ein **Vorkommen** einer Buchungs*art* auf der Zeitachse des Laufs — was die fenster-übergreifende
/// Query liefert (ADR-0013, Entscheidung E, Reichweite 2).
///
/// Es trägt **genau vier** Angaben, und jede hat einen Abnehmer in der Wiederkehr-Erkennung
/// ([0005](../../../../docs/issues/0005-wiederkehr-erkennung.md), die hierauf aufbaut): der
/// [`Fingerprint`] gruppiert die Vorkommen zu einer Art, der [`DayStamp`] trägt den **Rhythmus**
/// (die Abstände zwischen den Tagen), und [`Amount`] + [`Direction`] tragen den **Betragsvergleich**
/// — an dem eine Preiserhöhung sichtbar wird. Mehr mitzunehmen hieße Daten ohne Leser zu führen.
///
/// **Ein Wert, keine Entität:** ein Vorkommen wird nie adressiert, nie abgelegt und nie
/// fortgeschrieben — es entsteht bei jeder Query neu aus den unveränderlichen Aggregaten. Genau
/// darum gibt es hier **kein** Vorkommens-/Fold-Log.
///
/// **Datenschutz:** der [`Fingerprint`] trägt nur, was schon stromabwärts der Obfuskierungs-Grenze
/// steht — die öffentliche Gläubiger-ID oder das seed-basierte Pseudonym, nie einen Rohwert.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Occurrence {
    /// Die Feldreihenfolge ist zugleich die **Sortier-Ordnung** des abgeleiteten [`Ord`]: erst der
    /// Tag (die Zeitachse), dann der Schlüssel, dann die Zahlen. Sie ist **total**, also ist die
    /// Reihenfolge der Vorkommen eingabe-unabhängig — die Query antwortet auf erneute Zustellung
    /// **identisch**, ohne dass es dafür Dedupe-Maschinerie bräuchte.
    day: DayStamp,
    fingerprint: Fingerprint,
    direction: Direction,
    amount: Amount,
}

impl Occurrence {
    #[must_use]
    pub(crate) fn new(
        day: DayStamp,
        fingerprint: Fingerprint,
        direction: Direction,
        amount: Amount,
    ) -> Self {
        Self {
            day,
            fingerprint,
            direction,
            amount,
        }
    }

    #[must_use]
    pub fn day(&self) -> DayStamp {
        self.day
    }

    #[must_use]
    pub fn fingerprint(&self) -> &Fingerprint {
        &self.fingerprint
    }

    #[must_use]
    pub fn direction(&self) -> Direction {
        self.direction
    }

    #[must_use]
    pub fn amount(&self) -> Amount {
        self.amount
    }
}

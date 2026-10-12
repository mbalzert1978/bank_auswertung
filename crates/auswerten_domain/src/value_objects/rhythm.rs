use crate::value_objects::ChargeSeries;

/// Das **Abstandsband** eines monatlichen Rhythmus, in Tagen, beide Grenzen einschließlich.
///
/// Ein Kalendermonat ist 28 bis 31 Tage lang, und eine Lastschrift rutscht über ein Wochenende oder
/// einen Feiertag um bis zu drei Bankarbeitstage. `25..=35` deckt beides ab — und liegt weit genug
/// von der Woche (7) und vom Quartal (~91) entfernt, dass kein anderer Rhythmus hineinfällt.
const MONTHLY_DAYS: (i64, i64) = (25, 35);

/// Das **Abstandsband** eines jährlichen Rhythmus, in Tagen, beide Grenzen einschließlich: 365 oder
/// 366 Tage, dazu dieselbe Verschiebung um Wochenenden und Feiertage — großzügig gerundet, weil
/// zwischen Halbjahr (~182) und Jahr nichts anderes liegt, das hier hineinfallen könnte.
const YEARLY_DAYS: (i64, i64) = (350, 380);

/// Wie viele Abbuchungen ein **monatlicher** Rhythmus mindestens braucht, um als Wiederkehr zu gelten.
///
/// **Drei, nicht zwei:** zwei Käufe beim selben Händler im Monatsabstand sind schnell Zufall. Erst der
/// dritte auf demselben Takt ist ein Muster — und eine Historie von ~3 Jahren liefert ihn für jedes
/// echte Monatsabo mühelos.
const MONTHLY_MIN_CHARGES: usize = 3;

/// Wie viele Abbuchungen ein **jährlicher** Rhythmus mindestens braucht: **zwei** — mehr kann es nicht
/// sein. Die Historie reicht ~3 Jahre zurück, ein Jahresabo erscheint darin zwei- bis dreimal; wer
/// drei verlangte, übersähe es meistens (ADR-0013, Entscheidung E: „2+ Vorkommen im
/// 12-Monats-Abstand").
const YEARLY_MIN_CHARGES: usize = 2;

/// Der **Takt** eines wiederkehrenden Abflusses ([Issue 0005](../../../../docs/issues/0005-wiederkehr-erkennung.md)).
///
/// Ein **geschlossener Summentyp** (ADR-0005): genau die beiden Takte, nach denen das Ticket fragt.
/// Ein Quartals- oder Halbjahres-Takt wäre eine weitere Variante, und der Compiler wiese dann jede
/// Stelle aus, die sie noch nicht kennt — statt dass ein `Option<u32>`-Abstand irgendwo still durchliefe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Rhythm {
    Monthly,
    Yearly,
}

impl Rhythm {
    /// Der Takt einer Abbuchungs-Reihe — oder `None`, wenn sie keinen trägt. Die Reihe ist per
    /// Typ chronologisch ([`ChargeSeries`]), die Abstände sind also nie negativ.
    ///
    /// **Jeder** Abstand muss in **dasselbe** Band fallen; ein einziger Ausreißer bricht die Reihe. Das
    /// ist bewusst streng: ein ausgelassener Monat (Abstand ~60) oder eine doppelte Abbuchung am selben
    /// Tag (Abstand 0) sind kein Takt mehr, sondern eine Aussage, die diese Erkennung nicht treffen
    /// kann. Lieber ein Abo **nicht** erkennen als einen Rhythmus behaupten, den es nicht gibt.
    #[must_use]
    pub(crate) fn of(series: &ChargeSeries) -> Option<Self> {
        let mut gaps = series
            .days()
            .zip(series.days().skip(1))
            .map(|(earlier, later)| later.value().saturating_sub(earlier.value()));
        let rhythm = Self::of_gap(gaps.next()?)?;
        (gaps.all(|gap| Self::of_gap(gap) == Some(rhythm)) && series.len() >= rhythm.min_charges())
            .then_some(rhythm)
    }

    /// Wie oft im Jahr dieser Takt abbucht — der Faktor vom typischen Betrag zum **Jahresbetrag**.
    #[must_use]
    pub(crate) const fn charges_per_year(self) -> u64 {
        match self {
            Rhythm::Monthly => 12,
            Rhythm::Yearly => 1,
        }
    }

    const fn min_charges(self) -> usize {
        match self {
            Rhythm::Monthly => MONTHLY_MIN_CHARGES,
            Rhythm::Yearly => YEARLY_MIN_CHARGES,
        }
    }

    /// Der Takt, in dessen Band **ein** Abstand fällt.
    fn of_gap(days: i64) -> Option<Self> {
        let within = |(low, high): (i64, i64)| (low..=high).contains(&days);
        match (within(MONTHLY_DAYS), within(YEARLY_DAYS)) {
            (true, _) => Some(Rhythm::Monthly),
            (false, true) => Some(Rhythm::Yearly),
            (false, false) => None,
        }
    }
}

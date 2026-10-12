use std::cmp::Reverse;
use std::collections::BTreeMap;

use crate::value_objects::{
    AccountToken, Amount, BookingKind, Charge, ChargeSeries, Fingerprint, Recurrence, Share,
    WindowedBooking,
};

/// Die Tage eines Jahres — die Bezugsgröße der Hochrechnung auf den **Jahresbetrag**.
const DAYS_PER_YEAR: u64 = 365;

/// Ein **Kostentreiber** — das, worunter das Pareto-Ranking die Abflüsse des Laufs zusammenfasst
/// ([Issue 0007](../../../../docs/issues/0007-aderlass-pareto-ranking-und-spar-kandidaten.md)).
///
/// Ein **Summentyp** (ADR-0005), weil ein Abfluss auf genau drei Arten einem Treiber zufällt und
/// jede ihren eigenen Schlüssel hat:
///
/// - **eine Buchungsart** — über ihren [`Fingerprint`]; ein Händler hinter einem Durchleiter ist darum
///   ein eigener Treiber;
/// - **die Gebühren eines Kontos** — was die Bank auf **einem** Konto selbst einzieht
///   ([`BookingKind::Fee`](crate::BookingKind)), ein Treiber je belastetem Konto, wie in der
///   Gebühren-Sicht. Die Gebühr gewinnt vor dem Fingerabdruck: eine Entgeltzeile ist Geld, das die
///   Bank nimmt, auch wenn sie eine Gegenpartei nennt;
/// - **ohne Gegenpartei** — ein Abfluss, der weder eine Art noch eine Gebühr ist (etwa eine
///   Barabhebung). Er bekommt **einen** Sammel-Treiber statt keinen: fiele er weg, ergäben die
///   Anteile des Rankings nicht mehr das Ganze, und der Bericht verschwiege Geld, das abfloss.
///
/// **Der eine Gruppierungsschlüssel der genetteten Abflüsse:** Wiederkehr und Ranking gruppieren
/// beide nach ihm (`DriverGroups`). Stünden zwei Schlüssel nebeneinander, könnte eine einzelne
/// Entgeltzeile die Abo-Reihe ihrer Gegenpartei zerreißen, und eine monatliche Kontoführung verlöre
/// ihren Takt. So ist auch eine Gebühr wiederkehrend, wenn sie im Takt abgeht.
///
/// Die abgeleitete Ordnung (Art < Gebühr < Sammel-Treiber, darin nach Schlüssel) ist der stabile
/// Gleichstand-Entscheider des Rankings.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CostDriver {
    Kind(Fingerprint),
    Fee(AccountToken),
    Unattributed,
}

impl CostDriver {
    /// Der Treiber, dem **ein** Abfluss zufällt — die reine Zuordnung, siehe die Typ-Doku: die
    /// Gebühr vor der Art, die Art vor dem Sammel-Treiber.
    #[must_use]
    pub(crate) fn of(booking: &WindowedBooking) -> Self {
        match (booking.kind(), Fingerprint::of(booking.counterparty())) {
            (BookingKind::Fee, _) => Self::Fee(booking.ordering_account().clone()),
            (BookingKind::Regular, Some(fingerprint)) => Self::Kind(fingerprint),
            (BookingKind::Regular, None) => Self::Unattributed,
        }
    }

    /// Ist dieser Treiber **bestimmt** — eine Art oder die Gebühren eines Kontos —, nicht der Rest?
    ///
    /// Nur ein bestimmter Treiber hat einen Takt und kann Spar-Kandidat sein. Der Sammel-Treiber
    /// fasst Unverwandtes zusammen: ein „Rhythmus" über Barabhebungen und Einzelposten wäre Zufall,
    /// und „spar an allem ohne Gegenpartei" ist kein Vorschlag, auf den man handeln könnte.
    #[must_use]
    pub(crate) fn is_specific(&self) -> bool {
        !matches!(self, Self::Unattributed)
    }
}

/// Ein **Platz im Pareto-Ranking**: ein Kostentreiber, was über ihn im ganzen Lauf abfloss, welchen
/// Anteil das an allen Abflüssen hat und — kumuliert — an allen bis einschließlich dieses Platzes.
///
/// **Genettet:** Umbuchungen auf die eigenen Konten stehen in keinem Treiber; sie sind kein
/// Abfluss. Die Summe aller Treiber ist darum genau der Abfluss des genetteten Cashflows.
///
/// Dazu der **Jahresbetrag** — die Bezugsgröße der Spar-Kandidaten, auf der ein Monatsabo und ein
/// Jahresabo vergleichbar werden: für einen **wiederkehrenden** Treiber der aus der
/// [`Recurrence`] (aktueller Preis × Abbuchungen je Jahr), für jeden anderen der **Durchschnitt je
/// Jahr** über den beobachteten Zeitraum (siehe `annual_average`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RankedDriver {
    driver: CostDriver,
    outflow: Amount,
    bookings: usize,
    share: Share,
    cumulative_share: Share,
    annual_amount: Amount,
    recurrence: Option<Recurrence>,
}

impl RankedDriver {
    #[must_use]
    pub fn driver(&self) -> &CostDriver {
        &self.driver
    }

    /// Was über diesen Treiber im **ganzen Lauf** abfloss.
    #[must_use]
    pub fn outflow(&self) -> Amount {
        self.outflow
    }

    /// Wie viele Abflüsse diesen Treiber bilden.
    #[must_use]
    pub fn bookings(&self) -> usize {
        self.bookings
    }

    /// Sein Anteil an allen genetteten Abflüssen des Laufs — als Differenz der kumulierten Anteile
    /// zum Vorplatz, sodass die Anteile aller Plätze **genau** das Ganze ergeben.
    #[must_use]
    pub fn share(&self) -> Share {
        self.share
    }

    /// Der Anteil aller Treiber bis **einschließlich** diesem Platz — beim letzten das Ganze.
    #[must_use]
    pub fn cumulative_share(&self) -> Share {
        self.cumulative_share
    }

    /// Der **Jahresbetrag**: aus der Wiederkehr, wenn der Treiber eine ist, sonst der Durchschnitt
    /// je Jahr.
    #[must_use]
    pub fn annual_amount(&self) -> Amount {
        self.annual_amount
    }

    /// Die erkannte Wiederkehr dieses Treibers — Takt und gegebenenfalls Preissteigerung.
    #[must_use]
    pub fn recurrence(&self) -> Option<&Recurrence> {
        self.recurrence.as_ref()
    }
}

/// Die genetteten Abflüsse eines Laufs, **einmal** nach [`CostDriver`] gruppiert — die eine
/// Gruppierung, aus der Wiederkehr **und** Ranking stammen.
///
/// Je Treiber trägt sie, was das Ranking braucht (Summe und Zahl der Abflüsse), und die
/// [`Recurrence`], falls die Abflüsse eines **bestimmten** Treibers im Takt abgehen — ein Abo, ein
/// Dauerauftrag, eine monatliche Kontoführung. Der Sammel-Treiber bekommt keine. Geordnet nach dem
/// Treiber, also stabil und eingabe-unabhängig.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DriverGroups(Vec<DriverGroup>);

#[derive(Debug, Clone, PartialEq, Eq)]
struct DriverGroup {
    driver: CostDriver,
    outflow: Amount,
    bookings: usize,
    recurrence: Option<Recurrence>,
}

impl DriverGroups {
    /// Gruppiert die (schon genetteten) Abflüsse — **ein** Durchlauf über die Buchungen.
    pub(crate) fn of<'a>(outflows: impl Iterator<Item = &'a WindowedBooking>) -> Self {
        let by_driver = outflows.fold(
            BTreeMap::<CostDriver, Vec<Charge>>::new(),
            |mut by_driver, booking| {
                by_driver
                    .entry(CostDriver::of(booking))
                    .or_default()
                    .push(Charge::new(
                        booking.day(),
                        booking.amount(),
                        booking.ordering_account().clone(),
                    ));
                by_driver
            },
        );
        Self(
            by_driver
                .into_iter()
                .map(|(driver, charges)| DriverGroup {
                    outflow: charges
                        .iter()
                        .fold(Amount::default(), |sum, charge| sum.plus(charge.amount())),
                    bookings: charges.len(),
                    recurrence: driver
                        .is_specific()
                        .then(|| ChargeSeries::of(charges))
                        .flatten()
                        .and_then(|series| Recurrence::detect(driver.clone(), &series)),
                    driver,
                })
                .collect(),
        )
    }

    /// Die **wiederkehrenden** Treiber, in der Ordnung des Treibers.
    pub(crate) fn recurrences(&self) -> Vec<Recurrence> {
        self.0
            .iter()
            .filter_map(|group| group.recurrence.clone())
            .collect()
    }

    /// **Rankt die Treiber** — die reine Kernfunktion des Pareto-Rankings.
    ///
    /// Sortiert wird **absteigend nach Abfluss** und bei Gleichstand nach dem Treiber selbst, sodass
    /// dieselben Abflüsse in jeder Reihenfolge dasselbe Ranking ergeben.
    ///
    /// **Anteile ohne Rundungsrest:** gerundet wird nur der **kumulierte** Anteil (aus der kumulierten
    /// Summe); der Einzelanteil ist die Differenz zum kumulierten Anteil des Vorplatzes. So summieren
    /// sich die Einzelanteile **genau** zum Ganzen (10 000), und der letzte kumulierte Anteil ist es
    /// auch — getrennt abgerundete Einzelanteile verlören je Platz bis zu einen Basispunkt.
    ///
    /// `observed_days` ist die Länge des beobachteten Zeitraums in Tagen — die Grundlage des
    /// Jahresbetrags aller nicht wiederkehrenden Treiber.
    pub(crate) fn rank(&self, observed_days: u64) -> Vec<RankedDriver> {
        let total = self
            .0
            .iter()
            .fold(Amount::default(), |total, group| total.plus(group.outflow));
        let mut ordered = self.0.iter().collect::<Vec<_>>();
        ordered.sort_by_key(|group| (Reverse(group.outflow), &group.driver));
        ordered
            .into_iter()
            .scan(
                (Amount::default(), Share::of(Amount::default(), total)),
                |(cumulative, previous_share), group| {
                    *cumulative = cumulative.plus(group.outflow);
                    let cumulative_share = Share::of(*cumulative, total);
                    let share = cumulative_share.minus(*previous_share);
                    *previous_share = cumulative_share;
                    Some(RankedDriver {
                        driver: group.driver.clone(),
                        outflow: group.outflow,
                        bookings: group.bookings,
                        share,
                        cumulative_share,
                        annual_amount: group.recurrence.as_ref().map_or_else(
                            || annual_average(group.outflow, observed_days),
                            Recurrence::annual_amount,
                        ),
                        recurrence: group.recurrence.clone(),
                    })
                },
            )
            .collect()
    }
}

/// Der **Durchschnitt je Jahr** eines Betrags, der über `observed_days` Tage abfloss — der
/// Jahresbetrag jedes Treibers, der keine erkannte Wiederkehr ist (Einzelkäufe, unregelmäßige
/// Ausgaben, Gebühren, der Sammel-Treiber).
///
/// **Nie hochgerechnet, nur heruntergeteilt:** liegt weniger als ein Jahr vor, ist der Jahresbetrag
/// schlicht die Summe. Ein Einzelkauf von 500 € in einem Auszug über drei Monate auf 2 000 € im Jahr
/// hochzurechnen behauptete drei weitere Käufe, die niemand gesehen hat — der Spar-Kandidat stünde
/// auf einer erfundenen Zahl. Der Preis der Vorsicht: eine monatliche Gebühr in einem kurzen Auszug
/// erscheint mit weniger als ihrem vollen Jahr. Über mehr als ein Jahr wird geteilt: drei Jahre
/// Abflüsse sind ein Drittel davon je Jahr.
fn annual_average(outflow: Amount, observed_days: u64) -> Amount {
    let days = u128::from(observed_days.max(DAYS_PER_YEAR));
    let average =
        u128::from(outflow.minor_units()).saturating_mul(u128::from(DAYS_PER_YEAR)) / days;
    Amount::from_minor_units(u64::try_from(average).unwrap_or(outflow.minor_units()))
}

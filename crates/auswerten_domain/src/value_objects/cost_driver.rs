use std::cmp::Reverse;
use std::collections::BTreeMap;

use crate::value_objects::{
    AccountToken, Amount, BookingKind, Fingerprint, Recurrence, Share, WindowedBooking,
};

/// Die Tage eines Jahres — die Bezugsgröße der Hochrechnung auf den **Jahresbetrag**.
const DAYS_PER_YEAR: u64 = 365;

/// Ein **Kostentreiber** — das, worunter das Pareto-Ranking die Abflüsse des Laufs zusammenfasst
/// ([Issue 0007](../../../../docs/issues/0007-aderlass-pareto-ranking-und-spar-kandidaten.md)).
///
/// Ein **Summentyp** (ADR-0005), weil ein Abfluss auf genau drei Arten einem Treiber zufällt und
/// jede ihren eigenen Schlüssel hat:
///
/// - **eine Buchungsart** — über ihren [`Fingerprint`], den Schlüssel, unter dem auch die Wiederkehr
///   gruppiert; ein Händler hinter einem Durchleiter ist darum ein eigener Treiber;
/// - **die Gebühren eines Kontos** — was die Bank auf **einem** Konto selbst einzieht
///   ([`BookingKind::Fee`](crate::BookingKind)), ein Treiber je belastetem Konto, wie in der
///   Gebühren-Sicht. Die Gebühr gewinnt vor dem Fingerabdruck: eine Entgeltzeile ist Geld, das die
///   Bank nimmt, auch wenn sie eine Gegenpartei nennt;
/// - **ohne Gegenpartei** — ein Abfluss, der weder eine Art noch eine Gebühr ist (etwa eine
///   Barabhebung). Er bekommt **einen** Sammel-Treiber statt keinen: fiele er weg, ergäben die
///   Anteile des Rankings nicht mehr das Ganze, und der Bericht verschwiege Geld, das abfloss.
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

    /// Kann dieser Treiber ein **Spar-Kandidat** sein? Der Sammel-Treiber nicht: er ist keine Art,
    /// sondern der Rest — „spar an allem ohne Gegenpartei" ist kein Vorschlag, auf den man handeln
    /// könnte.
    #[must_use]
    pub(crate) fn is_actionable(&self) -> bool {
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

    /// Sein Anteil an allen genetteten Abflüssen des Laufs.
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

/// **Rankt die Abflüsse** nach Treibern — die reine Kernfunktion des Pareto-Rankings.
///
/// Je Treiber werden die Abflüsse summiert; sortiert wird **absteigend nach Betrag** und bei
/// Gleichstand nach dem Treiber selbst, sodass dieselben Abflüsse in jeder Reihenfolge dasselbe
/// Ranking ergeben. Der kumulierte Anteil wird aus der **kumulierten Summe** gerechnet, nicht aus
/// den gerundeten Einzelanteilen: so endet er am letzten Platz genau beim Ganzen, statt an
/// Rundungsresten unter 100 % hängen zu bleiben.
///
/// `observed_days` ist die Länge des beobachteten Zeitraums in Tagen — die Grundlage des
/// Jahresbetrags aller nicht wiederkehrenden Treiber.
pub(crate) fn rank(
    outflows: impl Iterator<Item = (CostDriver, Amount)>,
    recurrences: &[Recurrence],
    observed_days: u64,
) -> Vec<RankedDriver> {
    let by_driver = outflows.fold(
        BTreeMap::<CostDriver, (Amount, usize)>::new(),
        |mut by_driver, (driver, amount)| {
            let (sum, count) = by_driver.entry(driver).or_default();
            *sum = sum.plus(amount);
            *count = count.saturating_add(1);
            by_driver
        },
    );
    let total = by_driver
        .values()
        .fold(Amount::default(), |total, (sum, _)| total.plus(*sum));
    let recurring = recurrences
        .iter()
        .map(|recurrence| (recurrence.fingerprint(), recurrence))
        .collect::<BTreeMap<_, _>>();
    let mut ranked = by_driver.into_iter().collect::<Vec<_>>();
    ranked.sort_by(|(left, (left_sum, _)), (right, (right_sum, _))| {
        (Reverse(left_sum), left).cmp(&(Reverse(right_sum), right))
    });
    ranked
        .into_iter()
        .scan(
            Amount::default(),
            |cumulative, (driver, (outflow, bookings))| {
                *cumulative = cumulative.plus(outflow);
                let recurrence = match &driver {
                    CostDriver::Kind(fingerprint) => recurring.get(fingerprint).copied().cloned(),
                    CostDriver::Fee(_) | CostDriver::Unattributed => None,
                };
                Some(RankedDriver {
                    annual_amount: recurrence.as_ref().map_or_else(
                        || annual_average(outflow, observed_days),
                        Recurrence::annual_amount,
                    ),
                    driver,
                    outflow,
                    bookings,
                    share: Share::of(outflow, total),
                    cumulative_share: Share::of(*cumulative, total),
                    recurrence,
                })
            },
        )
        .collect()
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

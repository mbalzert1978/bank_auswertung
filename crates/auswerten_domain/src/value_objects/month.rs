/// Ein Buchungstag als **sortierbarer Tag-Stamp** (Tage seit Epoche, feste Zone —
/// [ADR-0013](../../../docs/adr/0013-zeitfenster-collector-paginierender-analyse-kopf.md),
/// Entscheidung C). Dieselbe Achse, auf der `fenstern` paginiert; hier ist sie der **Eingang** in
/// die Kalender-Sicht.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DayStamp(i64);

impl DayStamp {
    #[must_use]
    pub fn new(value: i64) -> Self {
        Self(value)
    }
}

/// Der **Monats-Bucket** des Cashflows — der *Zeitraum*, **innerhalb** dessen dieser Slice rechnet.
///
/// **Nicht das Fenster:** das Auswertungsfenster ist die **Seitengröße** des Pagers (~400 Tage) und
/// deckt viele Monate ab (CONTEXT.md, markierte Mehrdeutigkeit „Fenster vs. Zeitraum"). Genau diese
/// Kalender-Sicht rekonstruiert `auswerten` — `fenstern` rechnet auf der Achse nur `± Tage`.
///
/// **Ein Ordinal statt eines Paars** (`year * 12 + month − 1`): Jahres-/Monatswechsel sind damit
/// schlichte Nachbarschaft auf einer Achse, die natürliche `Ord` sortiert die Buckets
/// chronologisch, und die illegale Kombination „Monat 0/13" ist **unkonstruierbar** statt „geprüft"
/// (ADR-0005) — es gibt keinen Konstruktor, der sie erzeugen könnte.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Month(i64);

impl Month {
    /// Der Monat, **in den** ein Buchungstag fällt — der einzige Weg zu einem [`Month`]. Total: jeder
    /// Tag-Stamp liegt in genau einem Monat, es gibt nichts abzuweisen.
    #[must_use]
    pub(crate) fn of(day: DayStamp) -> Self {
        let (year, month) = civil_year_month(day.0);
        // Sättigend aus demselben Grund wie in `civil_year_month`: der Tag-Stamp kommt roh aus io.
        // `month − 1` liegt in `0..=11`, ist also für sich unkritisch.
        Self(year.saturating_mul(12).saturating_add(month - 1))
    }

    /// Das Kalenderjahr (proleptisch gregorianisch).
    #[must_use]
    pub fn year(self) -> i64 {
        self.0.div_euclid(12)
    }

    /// Der Monat im Jahr, **1..=12** — per Konstruktion, nicht per Prüfung.
    #[must_use]
    pub fn month_of_year(self) -> i64 {
        self.0.rem_euclid(12) + 1
    }
}

/// Rechnet Tage-seit-Epoche in `(Jahr, Monat)` des proleptisch gregorianischen Kalenders um —
/// die Kalender-Sicht, die die Domäne **selbst** rekonstruiert, weil sie dependency-frei ist und
/// keine Kalender-Crate binden darf (ADR-0007).
///
/// Es ist Howard Hinnants `civil_from_days`: es verschiebt den Nullpunkt auf den 1. März 0000, sodass
/// der Schalttag ans **Ende** des Jahres rutscht und die Schaltjahr-Fallunterscheidung ganz entfällt.
/// Danach ist alles ganzzahlige Division — und **total**: jeder Tag-Stamp liefert ein Ergebnis, es
/// gibt keinen Fehler-Ausgang. Der Tagesanteil wird nicht gebraucht und darum nicht berechnet.
///
/// **Panikfrei für jeden `i64`, exakt im realen Wertebereich** (Tag-Stamps aus Kontoauszügen). Der
/// Tag-Stamp kommt **roh aus io** — die Domäne hat keine Zusage, dass er plausibel ist —, darum
/// sättigen die **zwei** Stellen, an denen ein extremer Eingabewert überhaupt überlaufen kann: die
/// Nullpunkt-Verschiebung (nahe `i64::MAX`) und der Negativ-Zweig der Ära (nahe `i64::MIN`). Alles
/// dazwischen ist **beweisbar beschränkt** — `day_of_era` liegt in `0..=146_096`, `year_of_era` in
/// `0..=399`, `day_of_year` in `0..=365` —, dort wäre sättigende Arithmetik nur Rauschen ohne
/// erreichbaren Fall. Ein absurder Tag-Stamp liefert so einen absurden, aber **beschränkten** Monat
/// statt eines Panics im Debug-Build; dieselbe Taktik wie `fenstern_domain::WindowSpan::page`.
fn civil_year_month(days: i64) -> (i64, i64) {
    // Nullpunkt auf den 1. März 0000 schieben (Epoche + 719_468 Tage).
    let shifted = days.saturating_add(719_468);
    // Die 400-Jahres-Ära (146_097 Tage) — der Zyklus, nach dem sich der Kalender exakt wiederholt.
    let era = if shifted >= 0 {
        shifted
    } else {
        shifted.saturating_sub(146_096)
    } / 146_097;
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    // Der März-basierte Monatsindex 0..=11 und zurück auf den Januar-basierten 1..=12.
    let shifted_month = (5 * day_of_year + 2) / 153;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    // Januar/Februar gehören schon zum **folgenden** Kalenderjahr der März-basierten Zählung.
    (if month <= 2 { year + 1 } else { year }, month)
}

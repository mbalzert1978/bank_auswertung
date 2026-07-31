use std::collections::{BTreeMap, BTreeSet};

use crate::entities::AnalysisWindow;
use crate::value_objects::WindowedBooking;
use crate::value_objects::{
    AccountRole, Category, CategoryAnswer, CounterAccount, Currency, Fingerprint, InternalTransfer,
    Month, MonthlyCashflow, OwnAccounts, WindowId,
};

/// Das **Pro-Fenster-Ergebnis** — was dieser Slice aus **einem** eingefrorenen Auswertungsfenster
/// rechnet und ablegt (ADR-0013, Entscheidung E, Reichweite 1). Es trägt die erste periodische
/// Kennzahl — den **Cashflow je Monat innerhalb** des Fensters — und zwar **genettet**: eine
/// Umbuchung zwischen eigenen Konten steht nicht darin, sondern daneben in den
/// [`transfers`](Self::transfers). Hier docken die weiteren Fach-Analyzer an
/// (Gebühren-Klassifikation, Pareto).
///
/// Es referenziert sein Quell-Fenster **nur per ID** (ADR-0012) — die Buchungen bleiben, wo sie
/// eingefroren sind. Seine **eigene** Identität vergibt die Persistenz beim Ablegen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowEvaluation {
    window: WindowId,
    currency: Currency,
    months: Vec<MonthlyCashflow>,
    transfers: Vec<InternalTransfer>,
    fingerprints: Vec<Fingerprint>,
    counter_accounts: Vec<CounterAccount>,
    categories: Vec<CategoryAnswer>,
}

impl WindowEvaluation {
    /// **Wertet das Fenster aus** — der einzige Konstruktor und der fachliche Kern des Handlers:
    /// jede Buchung fällt in den Monat ihres Buchungstags und wächst dort die Spalte ihrer Richtung.
    ///
    /// **Zuvor entscheidet sich, in welche der beiden Sichten sie fällt** ([Issue
    /// 0004](../../../../docs/issues/0004-umbuchung-netting.md)): eine Buchung auf ein **eigenes**
    /// Gegenkonto ist eine Umbuchung und gehört zu den [`transfers`](Self::transfers), jede andere in
    /// den Cashflow. Die beiden Sichten sind darum disjunkt und zusammen vollständig — keine Buchung
    /// zählt doppelt, keine fällt heraus. Die Fingerabdruck- und die Gegenkonto-Liste bleiben davon
    /// **unberührt**: sie sagen, welche *Arten* im Fenster vorkamen, und eine Umbuchung kam vor.
    ///
    /// **Ein Fenster deckt mehrere Monate ab** — es ist die *Seitengröße* des Pagers (~400 Tage),
    /// nicht der Bucketing-*Zeitraum* (CONTEXT.md, „Fenster vs. Zeitraum"). Die `BTreeMap` fällt
    /// darum genau richtig: sie legt die Buckets **chronologisch** ab (der [`Month`] ist ein
    /// sortierbares Ordinal) und faltet gleiche Monate zusammen, ohne dass irgendwo sortiert oder
    /// gesucht werden müsste. Monate **ohne** Buchung entstehen gar nicht erst — ein leerer Bucket
    /// wäre eine Behauptung über einen Zeitraum, aus dem nichts vorliegt.
    #[must_use]
    pub(crate) fn of(
        window: WindowId,
        snapshot: &AnalysisWindow,
        own_accounts: &OwnAccounts,
        categories: Vec<CategoryAnswer>,
    ) -> Self {
        // Der genettete Cashflow: alles, was **nicht** zwischen den eigenen Konten floss. Der
        // Schlüssel ist allein der Monat — die Einheit `()` daneben ist der Preis dafür, dass sich
        // beide Sichten dieselbe Bucketing-Mechanik teilen (siehe [`bucket_by_month`]).
        let months = bucket_by_month(
            snapshot
                .bookings()
                .iter()
                .filter(|booking| transfer_role(booking, own_accounts).is_none())
                .map(|booking| ((), booking)),
        );
        // Die andere Seite: dieselben Monats-Buckets, aber je **Rolle des Gegenkontos** getrennt —
        // ein Abfluss aufs Sparkonto und einer aufs Haushaltskonto sind nicht dieselbe Bewegung.
        let transfers = bucket_by_month(
            snapshot
                .bookings()
                .iter()
                .filter_map(|booking| Some((transfer_role(booking, own_accounts)?, booking))),
        );
        // Die **distinkten** Fingerabdrücke des Fensters: der Schlüssel steht für eine *Art* von
        // Buchung, nicht für die einzelne — zwanzig Buchungen desselben Händlers sind **eine** Art.
        // Das `BTreeSet` faltet sie darum zusammen und legt sie zugleich in einer stabilen,
        // eingabe-unabhängigen Reihenfolge ab. Eine Buchung ohne Gegenpartei trägt keinen Schlüssel
        // bei (`None`).
        let fingerprints = snapshot
            .bookings()
            .iter()
            .filter_map(|booking| Fingerprint::of(booking.recipient(), booking.purpose()))
            .collect::<BTreeSet<_>>();
        // Die **distinkten** Einordnungen der Gegenkonten dieses Fensters (ADR-0013, Entscheidung
        // F) — gebildet auf **Token**, nie auf einer IBAN. Wie beim Fingerabdruck faltet das
        // `BTreeSet` sie zusammen und legt sie in stabiler, eingabe-unabhängiger Reihenfolge ab:
        // zwanzig Umbuchungen aufs selbe Sparkonto sind **eine** Einordnung. Eine Buchung **ohne**
        // Gegenkonto (Entgeltzeile) trägt nichts bei — „keine Gegenseite" ist etwas anderes als
        // „eine fremde Gegenseite", und das Ergebnis behauptet nur, was vorliegt.
        let counter_accounts = snapshot
            .bookings()
            .iter()
            .filter_map(WindowedBooking::counter_account)
            .map(|token| own_accounts.resolve(token))
            .collect::<BTreeSet<_>>();
        Self {
            window,
            currency: snapshot.currency().clone(),
            months: months.into_values().collect(),
            transfers: transfers
                .into_iter()
                .map(|((_month, role), flow)| InternalTransfer::of(role, flow))
                .collect(),
            fingerprints: fingerprints.into_iter().collect(),
            counter_accounts: counter_accounts.into_iter().collect(),
            categories: unambiguous(categories),
        }
    }

    #[must_use]
    pub fn window(&self) -> &WindowId {
        &self.window
    }

    #[must_use]
    pub fn currency(&self) -> &Currency {
        &self.currency
    }

    /// Die Monats-Buckets des **genetteten** Cashflows in **chronologischer** Reihenfolge — was
    /// wirklich zu- und abfloss. Umbuchungen zwischen den eigenen Konten stehen hier **nicht**; sie
    /// sind kein verlorenes Geld (CONTEXT.md, „Umbuchung") und stünden der Frage „wo bluten wir" nur
    /// als Scheinausgabe im Weg. Sichtbar bleiben sie in [`transfers`](Self::transfers), und der
    /// Brutto-Fluss eines Monats ist die Summe aus beiden.
    #[must_use]
    pub fn months(&self) -> &[MonthlyCashflow] {
        &self.months
    }

    /// Die **Umbuchungen** dieses Fensters — je Monat und Rolle des Gegenkontos ein Bucket, in
    /// chronologischer Reihenfolge und darin nach Rolle. Die andere Seite des Nettings: was aus dem
    /// Cashflow herausfiel, ist hier nachlesbar, statt verloren zu sein.
    #[must_use]
    pub fn transfers(&self) -> &[InternalTransfer] {
        &self.transfers
    }

    /// Die **distinkten** Fingerabdrücke der Buchungsarten dieses Fensters, in stabiler Reihenfolge
    /// — der Andockpunkt für die Wiederkehr-Erkennung, die gelernten Korrekturen und die
    /// Deduplizierung der Kategorisierer-Anfrage.
    #[must_use]
    pub fn fingerprints(&self) -> &[Fingerprint] {
        &self.fingerprints
    }

    /// Die **distinkten** Einordnungen der Gegenkonten dieses Fensters, in stabiler Reihenfolge —
    /// jede entweder eigen (samt Rolle) oder fremd. Auf ihr ruht das Umbuchungs-Netting: was hier als
    /// eigen erscheint, ist im Cashflow nicht mehr und in den [`transfers`](Self::transfers) dafür
    /// schon. Sie ist eine Liste von **Arten**, keine von Beträgen — zwanzig Umbuchungen aufs selbe
    /// Sparkonto sind hier ein Eintrag und dort ein aufsummierter Bucket.
    #[must_use]
    pub fn counter_accounts(&self) -> &[CounterAccount] {
        &self.counter_accounts
    }

    /// Die Kategorien, die das **Orakel** zu den zuvor **unbekannten** Buchungsarten dieses
    /// Fensters beigetragen hat — eine **Abbildung** Fingerabdruck → Kategorie: je Schlüssel
    /// höchstens ein Etikett, in stabiler Reihenfolge (siehe `unambiguous`; ein widersprüchlich
    /// gedeuteter Schlüssel trägt gar keines).
    ///
    /// **Nur das Neue steht hier, nicht das Bekannte.** Was das Kategorie-Register ohnehin schon
    /// weiß, noch einmal an jedes Fenster zu heften, hieße die gelernte Wahrheit zu duplizieren —
    /// dieselbe Linie, die dieser Slice beim Verzicht auf ein Vorkommens-/Fold-Log zieht.
    ///
    /// **Leer ist ein gültiges Ergebnis**, und zwar in beide Richtungen: es kann heißen, dass die
    /// Domäne jede Art des Fensters schon kannte — oder dass sie sie nicht deuten konnte.
    #[must_use]
    pub fn categories(&self) -> &[CategoryAnswer] {
        &self.categories
    }
}

/// Entscheidet für **eine** Buchung, ob sie eine Umbuchung ist: `Some(role)`, wenn ihr Gegenkonto in
/// der Eigen-Menge liegt — dann trägt sie die Rolle **dieses Gegenkontos** —, sonst `None`.
///
/// **Ein einzelnes Bein genügt als Nachweis.** Man könnte auf das *Paar* warten (Abgang hier,
/// Zugang dort) und nur netten, wenn beide vorliegen. Genau das wäre aber nur richtig, wenn beide
/// Konten im selben Lauf und im selben Fenster stünden: bei nur einem eingelesenen Auszug, an einer
/// Fenstergrenze oder bei einer Umbuchung über den Monatswechsel bliebe das Gegenstück aus, und die
/// Umbuchung zählte wieder als Ausgabe. Seit die Auflösung **auf Token** arbeitet (ADR-0013,
/// Entscheidung F), ist das Paar auch nicht nötig: „das Gegenkonto ist meines" steht schon an der
/// einen Zeile.
///
/// **Keine Gegenseite ist keine eigene.** Eine Entgeltzeile hat gar kein Gegenkonto — sie ist echter
/// Abfluss und bleibt im Cashflow (CONTEXT.md, „Umbuchungskosten": die Gebühr *für* eine Umbuchung
/// ist selbst keine).
fn transfer_role(booking: &WindowedBooking, own_accounts: &OwnAccounts) -> Option<AccountRole> {
    match booking
        .counter_account()
        .map(|token| own_accounts.resolve(token))
    {
        Some(CounterAccount::Own(role)) => Some(role),
        Some(CounterAccount::Foreign) | None => None,
    }
}

/// Faltet Buchungen zu **Monats-Buckets** — die eine Bucketing-Mechanik, die sich der genettete
/// Cashflow und die Umbuchungs-Sicht teilen: jede Buchung fällt in den Monat ihres Buchungstags und
/// wächst dort die Spalte ihrer Richtung.
///
/// Der `key` je Buchung ist, was die Sichten **zusätzlich** zum Monat trennt: nichts (`()`) beim
/// Cashflow, die Rolle des Gegenkontos bei den Umbuchungen. Er steht im Schlüssel **hinter** dem
/// Monat, sodass die `BTreeMap` zuerst chronologisch ordnet und erst danach nach ihm — dieselbe
/// stabile, eingabe-unabhängige Reihenfolge für beide.
fn bucket_by_month<'a, K: Ord>(
    bookings: impl Iterator<Item = (K, &'a WindowedBooking)>,
) -> BTreeMap<(Month, K), MonthlyCashflow> {
    bookings.fold(BTreeMap::new(), |mut buckets, (key, booking)| {
        let month = Month::of(booking.day());
        let bucket = buckets
            .entry((month, key))
            .or_insert_with(|| MonthlyCashflow::empty(month));
        *bucket = bucket.plus(booking.amount(), booking.direction());
        buckets
    })
}

/// Macht aus den Antworten des Orakels eine **Abbildung** Fingerabdruck → Kategorie: je Schlüssel
/// höchstens ein Etikett, in stabiler Reihenfolge.
///
/// **Ein Widerspruch ist keine Antwort.** Schickt das Orakel zu **einer** Art zwei **verschiedene**
/// Etiketten, hat es sie nicht gedeutet — es hat sich widersprochen. Eines davon auszuwählen (das
/// erste, das alphabetisch kleinste, irgendeines) wäre eine Deutung, die niemand vorgenommen hat;
/// sie ginge anschließend als *gelernte* Kategorie ins Register und sähe dort aus wie Wissen. Der
/// Schlüssel bekommt darum **gar kein** Etikett und bleibt unbekannt, sodass ein späterer Lauf
/// erneut fragen darf — lieber kein Ergebnis als ein falsches, das sich richtig liest (dieselbe
/// Linie wie beim Seed-Mismatch).
///
/// **Eine Wiederholung ist kein Widerspruch:** dieselbe Deutung mehrfach fällt zu einem Eintrag
/// zusammen (das innere `BTreeSet`) und geht nicht mit verloren.
///
/// **Terminal ist der Widerspruch nicht.** Das Orakel darf schweigen und es darf sich irren; die
/// Auswertung von seiner Auskunftsfreude abhängig zu machen, hieße es zum Kategorisierer-of-record
/// zu befördern (ADR-0004).
fn unambiguous(answers: Vec<CategoryAnswer>) -> Vec<CategoryAnswer> {
    answers
        .into_iter()
        .fold(
            BTreeMap::<Fingerprint, BTreeSet<Category>>::new(),
            |mut by_fingerprint, answer| {
                by_fingerprint
                    .entry(answer.fingerprint().clone())
                    .or_default()
                    .insert(answer.category().clone());
                by_fingerprint
            },
        )
        .into_iter()
        // Genau **ein** Etikett je Schlüssel überlebt; null wäre unmöglich, zwei sind der
        // Widerspruch oben.
        .filter_map(|(fingerprint, labels)| match labels.len() {
            1 => labels
                .into_iter()
                .next()
                .map(|category| CategoryAnswer::new(fingerprint, category)),
            _ => None,
        })
        .collect()
}

use std::collections::{BTreeMap, BTreeSet};

use crate::entities::AnalysisWindow;
use crate::value_objects::WindowedBooking;
use crate::value_objects::{
    Category, CategoryAnswer, CounterAccount, Currency, Fee, Fingerprint, InternalTransfer,
    MonthlyCashflow, VerifiedOwnAccounts, WindowId, bucket_by_month, fees, internal_transfers,
};

/// Das **Pro-Fenster-Ergebnis** — was dieser Slice aus **einem** eingefrorenen Auswertungsfenster
/// rechnet und ablegt (ADR-0013, Entscheidung E, Reichweite 1). Es trägt die erste periodische
/// Kennzahl — den **Cashflow je Monat innerhalb** des Fensters — und zwar **genettet**: eine
/// Umbuchung zwischen eigenen Konten steht nicht darin, sondern daneben in den
/// [`transfers`](Self::transfers) — und was die Bank innerhalb dieses Cashflows selbst einzog,
/// steht als [`fees`](Self::fees) noch einmal gesondert. Hier docken die weiteren Fach-Analyzer an
/// (Pareto, Wiederkehr).
///
/// **Die drei Sichten stehen nicht gleichrangig nebeneinander.** Cashflow und Umbuchungen
/// **teilen** die Buchungen unter sich auf (disjunkt, zusammen vollständig); die Gebühren
/// **überlagern** den Cashflow (ein Ausschnitt von ihm). Der Unterschied ist fachlich: eine
/// Umbuchung ist kein verlorenes Geld, eine Gebühr sehr wohl.
///
/// Es referenziert sein Quell-Fenster **nur per ID** (ADR-0012) — die Buchungen bleiben, wo sie
/// eingefroren sind. Seine **eigene** Identität vergibt die Persistenz beim Ablegen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowEvaluation {
    window: WindowId,
    currency: Currency,
    months: Vec<MonthlyCashflow>,
    transfers: Vec<InternalTransfer>,
    fees: Vec<Fee>,
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
    /// **Die Gebühren-Sicht teilt nichts auf, sie hebt heraus** ([Issue
    /// 0006](../../../../docs/issues/0006-gebuehren-klassifikation.md)): jede Buchung des Cashflows,
    /// deren Buchungstext sie als Bank-Entgelt ausweist, wächst **zusätzlich** einen Bucket je
    /// Monat und belastetem Konto. Sie verlässt den Cashflow dabei nicht — eine Gebühr ist echter
    /// Abfluss, im Gegensatz zur Umbuchung.
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
        own_accounts: VerifiedOwnAccounts<'_>,
        categories: Vec<CategoryAnswer>,
    ) -> Self {
        // Der genettete Cashflow: alles, was **nicht** zwischen den eigenen Konten floss. Der
        // Schlüssel ist allein der Monat — die Einheit `()` daneben ist der Preis dafür, dass sich
        // beide Sichten dieselbe Bucketing-Mechanik teilen (siehe [`bucket_by_month`]).
        let months = bucket_by_month(
            snapshot
                .bookings()
                .iter()
                .filter(|booking| booking.transfer_role(&own_accounts).is_none())
                .map(|booking| ((), booking)),
        );
        // Die andere Seite (je Rolle des Gegenkontos) und die Überlagerung (je belastetem Konto):
        // dieselben Funktionen, mit denen auch die fenster-übergreifende Reichweite sie rechnet.
        let transfers = internal_transfers(snapshot.bookings(), &own_accounts);
        let fees = fees(snapshot.bookings(), &own_accounts);
        // Die **distinkten** Fingerabdrücke des Fensters: der Schlüssel steht für eine *Art* von
        // Buchung, nicht für die einzelne — zwanzig Buchungen desselben Händlers sind **eine** Art.
        // Das `BTreeSet` faltet sie darum zusammen und legt sie zugleich in einer stabilen,
        // eingabe-unabhängigen Reihenfolge ab. Eine Buchung ohne Gegenpartei trägt keinen Schlüssel
        // bei (`None`).
        let fingerprints = snapshot
            .bookings()
            .iter()
            .filter_map(|booking| Fingerprint::of(booking.counterparty()))
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
            transfers,
            fees,
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

    /// Die **Bank-Gebühren** dieses Fensters — je Monat und belastetem Konto ein Bucket, in
    /// chronologischer Reihenfolge und darin nach Konto-Token. Was die Bank selbst einzog, statt
    /// dessen, was der Nutzer ausgab ([Issue
    /// 0006](../../../../docs/issues/0006-gebuehren-klassifikation.md)).
    ///
    /// **Ein Ausschnitt der [`months`](Self::months), keine Ergänzung.** Anders als die
    /// [`transfers`](Self::transfers), die aus dem Cashflow herausfielen, stehen diese Beträge
    /// **auch** dort — eine Gebühr ist echter, verlorener Abfluss. Wer beides addiert, zählt sie
    /// doppelt; wer sie voneinander abzieht, erhält, was der Nutzer selbst veranlasst hat.
    #[must_use]
    pub fn fees(&self) -> &[Fee] {
        &self.fees
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

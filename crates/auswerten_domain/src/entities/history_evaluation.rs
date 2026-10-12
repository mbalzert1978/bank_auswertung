use std::collections::BTreeMap;

use crate::entities::RunHistory;
use crate::value_objects::{
    Charge, Direction, Fingerprint, Occurrence, OwnAccounts, Recurrence, WindowedBooking,
};

/// Das **Ergebnis der fenster-übergreifenden Reichweite** (ADR-0013, Entscheidung E, Reichweite 2):
/// die Vorkommen der ganzen Zeitachse des Laufs und die **wiederkehrenden Abflüsse**, die darin
/// stecken ([Issue 0005](../../../../docs/issues/0005-wiederkehr-erkennung.md)).
///
/// **Zurückgegeben, nie abgelegt:** kein Vorkommens-/Fold-Log, keine zweite persistierte Sicht. Es
/// entsteht bei jedem Lauf neu aus den unveränderlichen Aggregaten — und ist darum
/// replay-deterministisch, ohne Dedupe-Logik.
///
/// **Die beiden Sichten sehen verschieden viel, bewusst.** Die [`occurrences`](Self::occurrences)
/// sind die **ungefilterte** Zeitachse: jede Buchung mit Gegenpartei, in beiden Richtungen, auch die
/// Umbuchung. Die [`recurrences`](Self::recurrences) beantworten eine engere Frage — *was kostet mich
/// regelmäßig Geld?* — und sehen darum nur, was davon **Kosten-Treiber** sein kann: Abflüsse, die
/// nicht auf ein eigenes Konto gingen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryEvaluation {
    occurrences: Vec<Occurrence>,
    recurrences: Vec<Recurrence>,
}

impl HistoryEvaluation {
    /// **Wertet die Historie aus** — der einzige Konstruktor.
    ///
    /// Für die Wiederkehr geht jede Buchung durch zwei Filter, und beide sind fachlich:
    /// - **nur Abflüsse** — ein regelmäßiger Zufluss (das Gehalt) ist kein Kosten-Treiber;
    /// - **genettet** ([Issue 0004](../../../../docs/issues/0004-umbuchung-netting.md)) — ein
    ///   Dauerauftrag aufs eigene Sparkonto ist kein verlorenes Geld, sondern eine Umbuchung, und
    ///   dieselbe Regel, die ihn aus dem Cashflow nimmt, nimmt ihn hier aus der Wiederkehr.
    ///
    /// Was bleibt, wird **je Fingerabdruck** gruppiert — dem Schlüssel einer Buchungs*art*, der bei
    /// einem Zahlungsintermediär schon den Händler-Token trägt. Zwei Händler hinter demselben
    /// Durchleiter landen darum in zwei Gruppen, ohne dass hier etwas dafür geschieht. Eine Buchung
    /// **ohne** Fingerabdruck (Entgeltzeile ohne Gegenpartei) gehört zu keiner Art und fällt heraus;
    /// was die Bank regelmäßig einzieht, weist die Gebühren-Sicht der pro-Fenster-Reichweite aus.
    #[must_use]
    pub(crate) fn of(history: &RunHistory, own_accounts: &OwnAccounts) -> Self {
        let bookings = history.bookings();
        let mut occurrences = bookings
            .iter()
            .filter_map(occurrence_of)
            .collect::<Vec<_>>();
        // **Chronologisch** — die Zeitachse ist der Sinn dieser Sicht. Die Ordnung ist **total**
        // (Tag, dann Schlüssel, dann Zahlen), die Sortierung also eingabe-unabhängig.
        occurrences.sort();
        // Die `BTreeMap` gruppiert je Art und legt die Gruppen zugleich in stabiler,
        // eingabe-unabhängiger Reihenfolge ab — dieselben Aggregate ergeben dieselbe Liste.
        let recurrences = bookings
            .iter()
            .filter(|booking| booking.direction() == Direction::Outgoing)
            .filter(|booking| booking.transfer_role(own_accounts).is_none())
            .filter_map(|booking| {
                Some((
                    Fingerprint::of(booking.counterparty())?,
                    Charge::new(
                        booking.day(),
                        booking.amount(),
                        booking.ordering_account().clone(),
                    ),
                ))
            })
            .fold(
                BTreeMap::<Fingerprint, Vec<Charge>>::new(),
                |mut by_kind, (fingerprint, charge)| {
                    by_kind.entry(fingerprint).or_default().push(charge);
                    by_kind
                },
            )
            .into_iter()
            .filter_map(|(fingerprint, charges)| Recurrence::detect(fingerprint, &charges))
            .collect();
        Self {
            occurrences,
            recurrences,
        }
    }

    /// Die Vorkommen der ganzen Zeitachse, chronologisch — ungefiltert (siehe Typ-Doku).
    #[must_use]
    pub fn occurrences(&self) -> &[Occurrence] {
        &self.occurrences
    }

    /// Die **wiederkehrenden Abflüsse** des Laufs, je Buchungsart einer, in stabiler Reihenfolge des
    /// Fingerabdrucks. Ein Ranking (wer kostet am meisten?) ist bewusst **nicht** hier — das ist die
    /// Pareto-Frage ([Issue 0007](../../../../docs/issues/0007-aderlass-pareto-ranking-und-spar-kandidaten.md)),
    /// und der Jahresbetrag ist die Größe, nach der sie ranken kann.
    #[must_use]
    pub fn recurrences(&self) -> &[Recurrence] {
        &self.recurrences
    }
}

/// Verdichtet eine gelesene Buchung zu ihrem Vorkommen. Eine Buchung **ohne** Gegenpartei
/// (Entgeltzeile) trägt **keines** bei: es gibt nichts, was ihre Art unterscheidbar machte, und ein
/// Platzhalter-Schlüssel verklumpte alle Entgeltzeilen zu einer scheinbar wiederkehrenden Art.
fn occurrence_of(booking: &WindowedBooking) -> Option<Occurrence> {
    Fingerprint::of(booking.counterparty()).map(|fingerprint| {
        Occurrence::new(
            booking.day(),
            fingerprint,
            booking.direction(),
            booking.amount(),
        )
    })
}

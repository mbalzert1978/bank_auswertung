use crate::entities::WindowedBooking;
use crate::error::WindowError;
use crate::value_objects::{BookingId, Currency, WindowSpan};

/// Das **Auswertungsfenster** — das Aggregat, zu dem der Slice die N Buchungen einer Seite
/// **N→1 reduziert** und das er in eigener Ablage einfriert
/// ([ADR-0013](../../../docs/adr/0013-zeitfenster-collector-paginierender-analyse-kopf.md),
/// Entscheidung A). Es trägt seine Lauf-Spanne, die **eine** Währung und die Fremdschlüssel auf
/// genau die Zeilen, die zum Fenster gehören — die Zeilen selbst bleiben in der
/// `obfuskiert`-Ablage (Aggregate referenzieren Fremd-Entitäten **nur per ID**, ADR-0012). Seine
/// **eigene** Identität vergibt die Persistenz beim Ablegen.
///
/// **Die Invarianten sitzen an der Konstruktion, nicht in einer Prüfung danach** (ADR-0005):
/// [`open`](AnalysisWindow::open) ist der **einzige** Weg herein, die Felder sind privat — wer ein
/// `AnalysisWindow` in der Hand hält, hält damit den Beweis, dass es **nicht leer** ist und **eine
/// einzige Währung** trägt. Ein leeres oder währungsgemischtes Fenster ist unkonstruierbar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalysisWindow {
    span: WindowSpan,
    currency: Currency,
    bookings: Vec<BookingId>,
}

impl AnalysisWindow {
    /// **Öffnet** das Fenster über den Buchungen einer Seite — der einzige Konstruktor und damit die
    /// Stelle, an der beide Invarianten entstehen:
    ///
    /// - **nicht leer:** ohne erste Buchung gibt es kein Fenster ([`WindowError::EmptyWindow`]) —
    ///   jedes Fenster sitzt auf echten Daten (ADR-0013, Entscheidung B);
    /// - **eine Währung:** die Währung des Fensters *ist* die der ersten Buchung; schert eine
    ///   weitere aus, entsteht kein Fenster ([`WindowError::MixedCurrency`]) — die Domäne summiert
    ///   keine gemischten Währungen (ADR-0005).
    pub(crate) fn open(
        span: WindowSpan,
        bookings: Vec<WindowedBooking>,
    ) -> Result<Self, WindowError> {
        let (head, rest) = bookings.split_first().ok_or(WindowError::EmptyWindow)?;
        let currency = head.currency().clone();
        rest.iter()
            .all(|booking| booking.currency() == &currency)
            .then(|| Self {
                span,
                currency,
                bookings: bookings.iter().map(|b| b.id().clone()).collect(),
            })
            .ok_or(WindowError::MixedCurrency)
    }

    #[must_use]
    pub fn span(&self) -> WindowSpan {
        self.span
    }

    #[must_use]
    pub fn currency(&self) -> &Currency {
        &self.currency
    }

    /// Die eingefrorenen Fremdschlüssel auf die Zeilen der Seite, in Lesereihenfolge.
    #[must_use]
    pub fn bookings(&self) -> &[BookingId] {
        &self.bookings
    }
}

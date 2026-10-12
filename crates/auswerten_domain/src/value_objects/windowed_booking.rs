use crate::value_objects::{
    AccountRole, AccountToken, Amount, BookingKind, CounterAccount, Counterparty, DayStamp,
    Direction, NonEmptyString, OwnAccounts,
};

/// Eine **Buchung des eingefrorenen Fensters**, wie der Lese-Port
/// [`AnalysisWindowSource`](crate::AnalysisWindowSource) sie liefert — genau das, was dieser Slice
/// von ihr braucht: ihren Buchungstag (der den Monats-Bucket bestimmt), ihren vorzeichenlosen
/// Betrag und ihre Richtung für den Cashflow, ihre [`Counterparty`] für den
/// [`Fingerprint`](crate::Fingerprint) und ihren Buchungstext für die
/// [`BookingKind`](crate::BookingKind).
///
/// Ein **Wert**, keine Entität: dieser Slice adressiert nie eine einzelne Buchung, er reduziert über
/// sie. Ihre Identität bleibt als Fremdschlüssel im `Auswertungsfenster` von `fenstern` — hierher
/// mitzunehmen, was niemand liest, hieße Zustand ohne Leser zu führen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowedBooking {
    day: DayStamp,
    amount: Amount,
    direction: Direction,
    /// Wer auf der anderen Seite stand und wofür — die beiden reisen als **ein** Wert, weil sie nur
    /// gemeinsam etwas aussagen (siehe [`Counterparty`]).
    counterparty: Counterparty,
    /// Das Gegenkonto als **Token** — echt optional: eine Entgeltzeile hat keine Gegenseite, und
    /// „kein Gegenkonto" ist etwas anderes als „ein fremdes Gegenkonto".
    counter_account: Option<AccountToken>,
    /// Das **belastete** Konto als Token — im Gegensatz zum Gegenkonto **nicht** optional: jede
    /// Buchung steht auf genau einem Konto, sonst gäbe es sie nicht.
    ordering_account: AccountToken,
    /// Der von der **Bank vergebene** Buchungstext (`LASTSCHRIFT`, `ENTGELTABSCHLUSS`, …) — die
    /// Grundlage der [`BookingKind`](crate::BookingKind). Echt optional: nicht jede Zeile führt
    /// einen, und ohne ihn ist die Buchung schlicht keine erkennbare Gebühr.
    booking_text: Option<NonEmptyString>,
}

impl WindowedBooking {
    /// **Hydratisiert** die Buchung aus ihren bereits geprüften Teilen — keine frische Identität,
    /// sondern Rekonstitution einer stromaufwärts schon eingefrorenen Zeile.
    #[must_use]
    pub fn hydrate(
        day: DayStamp,
        amount: Amount,
        direction: Direction,
        counterparty: Counterparty,
        counter_account: Option<AccountToken>,
        ordering_account: AccountToken,
        booking_text: Option<NonEmptyString>,
    ) -> Self {
        Self {
            day,
            amount,
            direction,
            counterparty,
            counter_account,
            ordering_account,
            booking_text,
        }
    }

    pub(crate) fn counter_account(&self) -> Option<&AccountToken> {
        self.counter_account.as_ref()
    }

    /// Entscheidet für **eine** Buchung, ob sie eine Umbuchung ist: `Some(role)`, wenn ihr
    /// Gegenkonto in der Eigen-Menge liegt — dann trägt sie die Rolle **dieses Gegenkontos** —, sonst
    /// `None`.
    ///
    /// **Eine Regel für beide Reichweiten:** die pro-Fenster-Reichweite nettet damit den Cashflow
    /// (Issue 0004), die fenster-übergreifende die Wiederkehr (Issue 0005). Stünde sie zweimal da,
    /// könnte ein Dauerauftrag aufs Sparkonto im Cashflow fehlen und trotzdem als Abo auftauchen.
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
    /// ist selbst keine). Genau daran hängt, dass beide Analyzer koexistieren: die Umbuchung fällt
    /// heraus, die Gebühr für sie bleibt und wird von [`Fee`](crate::Fee) zusätzlich herausgehoben.
    pub(crate) fn transfer_role(&self, own_accounts: &OwnAccounts) -> Option<AccountRole> {
        match self
            .counter_account()
            .map(|token| own_accounts.resolve(token))
        {
            Some(CounterAccount::Own(role)) => Some(role),
            Some(CounterAccount::Foreign) | None => None,
        }
    }

    pub(crate) fn ordering_account(&self) -> &AccountToken {
        &self.ordering_account
    }

    /// Die **Buchungsart** dieser Zeile — abgeleitet, nicht gespeichert: sie ist eine Deutung des
    /// Buchungstexts, und die gehört der Domäne (ADR-0004), nicht der Ablage.
    pub(crate) fn kind(&self) -> BookingKind {
        BookingKind::of(self.booking_text.as_ref())
    }

    pub(crate) fn counterparty(&self) -> &Counterparty {
        &self.counterparty
    }

    pub(crate) fn day(&self) -> DayStamp {
        self.day
    }

    pub(crate) fn amount(&self) -> Amount {
        self.amount
    }

    pub(crate) fn direction(&self) -> Direction {
        self.direction
    }
}

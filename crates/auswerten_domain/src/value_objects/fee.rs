use crate::value_objects::{AccountToken, Amount, Month, MonthlyCashflow};

/// Eine **Bank-Gebühr** als Monats-Bucket: was die Bank in diesem Monat auf **einem** Konto selbst
/// eingezogen hat ([Issue 0006](../../../../docs/issues/0006-gebuehren-klassifikation.md)).
///
/// **Anders als die [`InternalTransfer`](crate::InternalTransfer) ist sie kein Gegenstück zum
/// Cashflow, sondern eine Überlagerung von ihm.** Eine Umbuchung ist kein Abfluss und fällt darum
/// aus dem Cashflow heraus; eine Gebühr ist echter, verlorener Abfluss und bleibt **darin** stehen.
/// Was hier steht, ist also ein **Teil** der Monats-Buckets, nicht etwas neben ihnen — sie zum
/// Cashflow zu addieren zählte dasselbe Geld zweimal.
///
/// Die Form ist trotzdem dieselbe wie dort ([`MonthlyCashflow`]), und zwar aus einem eigenen Grund:
/// eine Gebühr kann **erstattet** werden (Storno, Kulanz). Ein verrechneter Saldo verspielte den
/// Unterschied zwischen „die Bank hat nichts genommen" und „sie hat genommen und zurückgegeben" —
/// das Vorzeichen lebt in der Richtung (ADR-0005).
///
/// **Das Konto ist das belastete**, nicht ein Gegenkonto: eine Entgeltzeile hat keine Gegenseite,
/// die Bank bucht sie direkt aufs Konto. Es steht als opakes `KONTO_…`-Token da — die Frage, die es
/// beantwortbar macht, lautet „welches meiner Konten kostet mich Gebühren", und die braucht keine
/// IBAN, nur Unterscheidbarkeit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fee {
    account: AccountToken,
    flow: MonthlyCashflow,
}

impl Fee {
    #[must_use]
    pub(crate) const fn of(account: AccountToken, flow: MonthlyCashflow) -> Self {
        Self { account, flow }
    }

    /// Das **belastete** Konto als Token — das Auftragskonto der Zeile, nie ein Gegenkonto.
    #[must_use]
    pub fn account(&self) -> &AccountToken {
        &self.account
    }

    #[must_use]
    pub fn month(&self) -> Month {
        self.flow.month()
    }

    /// Was die Bank in diesem Monat **erstattet** hat — meist nichts, aber ein Storno ist keine
    /// negative Gebühr, sondern ein Zufluss.
    #[must_use]
    pub fn incoming(&self) -> Amount {
        self.flow.incoming()
    }

    /// Was die Bank in diesem Monat **eingezogen** hat — die Zahl, um die es geht.
    #[must_use]
    pub fn outgoing(&self) -> Amount {
        self.flow.outgoing()
    }
}

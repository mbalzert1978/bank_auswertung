use crate::value_objects::{AccountRole, AccountToken, Amount, Month, MonthlyCashflow};

/// Der **Cashflow eines eigenen Kontos in einem Monat** — der Aderlass je Konto und Monat über den
/// ganzen Lauf ([Issue 0007](../../../../docs/issues/0007-aderlass-pareto-ranking-und-spar-kandidaten.md)).
///
/// **Genettet, wie der Monats-Cashflow der pro-Fenster-Reichweite:** eine Umbuchung zwischen den
/// eigenen Konten steht nicht darin (sie ist kein verlorenes Geld), eine Gebühr schon (sie ist es).
/// Anders als dort ist der Bucket aber **je belastetem Konto** getrennt — die Frage, die er
/// beantwortet, lautet „welches meiner Konten blutet in welchem Monat", und über **alle** Fenster des
/// Laufs, weil ein Monat an einer Fenstergrenze sonst in zwei halbe Buckets zerfiele.
///
/// **Das Konto ist das belastete** (das Auftragskonto der Buchung), als opakes `KONTO_…`-Token. Seine
/// **Rolle** steht daneben, sofern die Eigene-Konten-Config das Konto führt — sie ist optional, weil
/// ein eingelesenes Konto, das die Config nicht nennt, trotzdem ein Konto mit Cashflow ist; ihm eine
/// Rolle anzudichten wäre eine Deutung, die niemand vorgenommen hat.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountCashflow {
    account: AccountToken,
    role: Option<AccountRole>,
    flow: MonthlyCashflow,
}

impl AccountCashflow {
    #[must_use]
    pub(crate) const fn of(
        account: AccountToken,
        role: Option<AccountRole>,
        flow: MonthlyCashflow,
    ) -> Self {
        Self {
            account,
            role,
            flow,
        }
    }

    /// Das **belastete eigene Konto** als Token.
    #[must_use]
    pub fn account(&self) -> &AccountToken {
        &self.account
    }

    /// Die Rolle dieses Kontos laut Eigene-Konten-Config — `None`, wenn die Config es nicht führt.
    #[must_use]
    pub fn role(&self) -> Option<AccountRole> {
        self.role
    }

    #[must_use]
    pub fn month(&self) -> Month {
        self.flow.month()
    }

    /// Was in diesem Monat auf diesem Konto **hereinkam** — ohne Umbuchungen von den eigenen Konten.
    #[must_use]
    pub fn incoming(&self) -> Amount {
        self.flow.incoming()
    }

    /// Was in diesem Monat von diesem Konto **abfloss** — ohne Umbuchungen auf die eigenen Konten,
    /// mit den Gebühren.
    #[must_use]
    pub fn outgoing(&self) -> Amount {
        self.flow.outgoing()
    }
}

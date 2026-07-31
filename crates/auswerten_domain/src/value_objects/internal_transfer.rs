use crate::value_objects::{AccountRole, Amount, Month, MonthlyCashflow};

/// Eine **Umbuchung** als Monats-Bucket: Geld, das zwischen den **eigenen** Konten verschoben wurde
/// (CONTEXT.md, „Umbuchung") — kein Abfluss und kein verlorenes Geld, darum aus dem Cashflow
/// herausgenettet ([Issue 0004](../../../../docs/issues/0004-umbuchung-netting.md)).
///
/// **Herausgenettet heißt nicht gelöscht.** Der Cashflow und diese Sicht sind zwei Seiten derselben
/// Sache: was hier steht, fehlt dort — und der Brutto-Fluss eines Monats ist die Summe aus beiden.
/// Genau darum trägt eine Umbuchung dieselbe Form wie der Bucket, aus dem sie verschwunden ist
/// ([`MonthlyCashflow`]), statt einer eigenen: hätte sie eine andere, ließe sich das Brutto nicht
/// mehr zusammenzählen, und „ausgeschlossen, aber sichtbar" wäre nur die halbe Zusage.
///
/// **Die Rolle ist die des Gegenkontos**, nicht die des Kontos, auf dem gebucht wurde: ein Abfluss
/// unter `Spar` ist Geld, das **aufs** Sparkonto ging, ein Zufluss unter `Spar` ist Geld, das **von**
/// dort kam. Die Rolle trägt die Deutung (CONTEXT.md, „Konto-Rolle") — ohne sie bliebe von einer
/// Umbuchung nur ein Betrag, aus dem sich nichts folgern ließe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InternalTransfer {
    role: AccountRole,
    flow: MonthlyCashflow,
}

impl InternalTransfer {
    #[must_use]
    pub(crate) const fn of(role: AccountRole, flow: MonthlyCashflow) -> Self {
        Self { role, flow }
    }

    /// Die Rolle des **Gegenkontos**, zwischen dem und dem gebuchten Konto das Geld floss.
    #[must_use]
    pub fn role(&self) -> AccountRole {
        self.role
    }

    #[must_use]
    pub fn month(&self) -> Month {
        self.flow.month()
    }

    /// Was in diesem Monat **von** dem Konto dieser Rolle hereinkam.
    #[must_use]
    pub fn incoming(&self) -> Amount {
        self.flow.incoming()
    }

    /// Was in diesem Monat **auf** das Konto dieser Rolle ging.
    #[must_use]
    pub fn outgoing(&self) -> Amount {
        self.flow.outgoing()
    }
}

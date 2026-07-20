use std::collections::BTreeMap;

use crate::value_objects::{AccountRole, AccountToken, CounterAccount, SeedMarker};

/// Die **Eigene-Konten-Config, token-orientiert** — die eine Auskunft, die dieser Slice braucht, um
/// ein Gegenkonto einzuordnen: welche `KONTO_…`-Token sind meine eigenen, und in welcher Rolle.
///
/// **Sie wird als Wert hereingereicht, nicht gelesen.** Die Domäne kennt keine Datei, kein Format
/// und keinen Pfad; wer die Config beschafft (io/Loader, [Issue 0002](../../../../docs/issues/0002-eigene-konten-config-schema-und-loader.md))
/// ist ihr gleichgültig. Sie bekommt fertige, geprüfte Werte — die Rolle als [`AccountRole`]-Variante,
/// nicht als Zeichenkette, die noch zu deuten wäre.
///
/// **Der Seed steht nicht hier.** Die Token sind bereits **mit demselben Seed** gebildet wie die
/// Gegenkonto-Token der Buchungen; dass das so ist, sichert der [`SeedMarker`](crate::SeedMarker) ab,
/// nicht dieser Typ. Der Seed selbst erreicht diesen Slice nie (ADR-0010).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnAccounts {
    /// Das Kennzeichen der Tokenisierung, aus der diese Token stammen — die Voraussetzung, unter
    /// der ein Vergleich mit den Gegenkonto-Token des Fensters überhaupt trägt.
    marker: SeedMarker,
    roles: BTreeMap<AccountToken, AccountRole>,
}

impl OwnAccounts {
    /// Baut die Eigen-Menge aus den bereits geprüften Paaren (Token, Rolle). **Infallibel:** die
    /// Prüfungen sind an der Grenze schon geführt — das Token ist nicht-leer, die Rolle ist eine der
    /// vier Varianten. Was hier ankommt, ist per Konstruktion gültig (_parse, don't validate_).
    ///
    /// Eine **leere** Config ist ausdrücklich zulässig und kein Fehler: wer keine eigenen Konten
    /// führt, hat schlicht nur fremde Gegenkonten — das ist eine Aussage, keine Lücke.
    #[must_use]
    pub fn of(
        marker: SeedMarker,
        roles: impl IntoIterator<Item = (AccountToken, AccountRole)>,
    ) -> Self {
        Self {
            marker,
            roles: roles.into_iter().collect(),
        }
    }

    /// Das Kennzeichen der Tokenisierung, aus der diese Eigen-Menge stammt.
    pub(crate) fn marker(&self) -> &SeedMarker {
        &self.marker
    }

    /// **Ordnet ein Gegenkonto ein** — der fachliche Kern dieses Tickets, eine reine Funktion auf
    /// Token: liegt das Token in der Eigen-Menge, ist es [`CounterAccount::Own`] **samt der dort
    /// hinterlegten Rolle**; jedes andere ist [`CounterAccount::Foreign`].
    #[must_use]
    pub(crate) fn resolve(&self, token: &AccountToken) -> CounterAccount {
        self.roles
            .get(token)
            .map_or(CounterAccount::Foreign, |role| CounterAccount::Own(*role))
    }
}

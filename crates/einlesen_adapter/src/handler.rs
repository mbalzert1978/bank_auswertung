use std::fmt;

use application_kernel::{Handler, Publisher};
use einlesen_domain::{Kalender, ZeilenAggregat, ZeilenError};
use shared_kernel::OutboxError;

use crate::Zeile;
use crate::eingelesen::Eingelesen;
use crate::ports::{Rohquelle, RohquelleError, Zeilenspeicher};
use crate::umsaetze_einlesen::UmsätzeEinlesen;

/// Orchestriert das Einlesen einer Quelle (liegt im **Adapter**, nicht in der blinden
/// Domäne — er bindet `application_kernel` und verdrahtet die Domäne an ihre Ports): liest
/// **alle** Rohzeilen aus der [`Rohquelle`], deutet je Zeile ein [`ZeilenAggregat`] und stagt
/// erst danach jedes gültige DTO **gemeinsam mit** genau einem ausgehenden `Eingelesen`-Command.
///
/// **Fehler bubblen auf, der Slice dead-lettert nicht selbst:** ein transienter Lesefehler wird
/// zu `Retry`; scheitert eine — von io bereits gelesene, strukturell valide — Zeile am Aggregat,
/// ist das **terminal** und muss reportet werden, also steigt der Handler mit dem Fehler aus
/// (er bubblet über die Consume-Bridge zu `OutboxError::Warning`, und die io-Outbox dead-lettert
/// die Nachricht). Weil erst **alle** Aggregate gebaut werden (`collect`) und dann gestaget wird,
/// hinterlässt ein terminaler Fehler nichts Halbes — die (io-seitige) Transaktion umschließt nur
/// die vollständig gültigen Aggregate.
pub(crate) struct EinleseHandler<'a> {
    pub(crate) rohquelle: &'a dyn Rohquelle,
    pub(crate) kalender: &'a dyn Kalender,
    pub(crate) zeilenspeicher: &'a dyn Zeilenspeicher,
    pub(crate) publisher: &'a dyn Publisher<Eingelesen>,
}

impl Handler for EinleseHandler<'_> {
    type Command = UmsätzeEinlesen;
    type Response = ();
    type Error = EinleseError;

    fn handle(&self, command: UmsätzeEinlesen) -> Result<Self::Response, Self::Error> {
        self.rohquelle
            .lies(&command.quelle)?
            .iter()
            .map(|rohsatz| ZeilenAggregat::aus_rohzeile(rohsatz, self.kalender))
            .collect::<Result<Vec<_>, _>>()?
            .iter()
            .for_each(|aggregat| {
                let zeile = Zeile::aus_aggregat(aggregat);
                self.zeilenspeicher.stage(&zeile);
                self.publisher.publish(Eingelesen::new(zeile));
            });
        Ok(())
    }
}

/// Der Fehler des Einlesens — vereint die zwei Quellen, die den Handler scheitern lassen, und
/// trägt so die **Disposition**: der transiente Lesefehler der Quelle (`RohquelleError`) und der
/// terminale Aggregat-Fehler einer Zeile (`ZeilenError`). Die Consume-Bridge mappt ihn über
/// `Into<OutboxError>` — Retry für die Quelle, Dead-Letter (`Warning`) für das Aggregat; die
/// Zuordnung ist damit eine Adapter-Entscheidung, keine Kernel-Regel.
#[derive(Debug)]
pub(crate) enum EinleseError {
    /// Transient: die Quelle war nicht lesbar → erneut zustellen.
    Quelle(RohquelleError),
    /// Terminal: eine gelesene Zeile ist am Aggregat ungültig → Dead-Letter.
    Zeile(ZeilenError),
}

impl From<RohquelleError> for EinleseError {
    fn from(fehler: RohquelleError) -> Self {
        Self::Quelle(fehler)
    }
}

impl From<ZeilenError> for EinleseError {
    fn from(fehler: ZeilenError) -> Self {
        Self::Zeile(fehler)
    }
}

impl fmt::Display for EinleseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EinleseError::Quelle(fehler) => write!(f, "{fehler}"),
            EinleseError::Zeile(fehler) => write!(f, "{fehler}"),
        }
    }
}

impl std::error::Error for EinleseError {}

impl From<EinleseError> for OutboxError {
    fn from(fehler: EinleseError) -> Self {
        match fehler {
            // Transient → Retry (delegiert an die Quellen-Zuordnung).
            EinleseError::Quelle(quelle) => quelle.into(),
            // Terminal → Dead-Letter; der Grund ist der **redigierte** ZeilenError (kein Rohwert),
            // sodass ein Spaltenverschub keine IBAN in die persistierte Meldung leakt.
            EinleseError::Zeile(zeile) => OutboxError::Warning {
                reason: zeile.to_string(),
            },
        }
    }
}

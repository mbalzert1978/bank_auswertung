use einlesen_domain::{NonEmptyString, Rohzeile};
use serde::{Deserialize, Serialize};

/// Der rohe Datensatz **einer** Zeile: die zehn verwerteten CSV-CAMT-v8-Felder als rohe Zellen
/// (leer/fehlend = `None`). Erfüllt den Domänen-Port `Rohzeile`, indem er jede Zelle über
/// `NonEmptyString` an die Grenze reicht — **wie** eine CSV-Spalte einem Feld zugeordnet wird,
/// kapselt er hier, die Domäne fordert nur an. Reine Wire-/Datenform (öffentliche Felder, analog
/// [`Zeile`](crate::Zeile)); Test-Fixtures (eine gültige Zeile + Feld-Overrides) leben **in den
/// Specs**, nicht hier — dieser Typ trägt keine Test-Ergonomie.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Rohsatz {
    pub auftragskonto: Option<String>,
    pub buchungstag: Option<String>,
    pub valuta: Option<String>,
    pub betrag: Option<String>,
    pub währung: Option<String>,
    pub empfänger_name: Option<String>,
    pub gläubiger_id: Option<String>,
    pub gegenkonto: Option<String>,
    pub verwendungszweck: Option<String>,
    pub buchungstext: Option<String>,
}

/// Reicht eine rohe Zelle als `NonEmptyString` an die Domänengrenze (leer/`None` → `None`); der
/// `NonEmptyString`-Kern trimmt und weist Leeres genau hier ab.
fn as_nonempty(roh: &Option<String>) -> Option<NonEmptyString> {
    roh.as_deref().and_then(NonEmptyString::new)
}

impl Rohzeile for Rohsatz {
    fn auftragskonto(&self) -> Option<NonEmptyString> {
        as_nonempty(&self.auftragskonto)
    }
    fn buchungstag(&self) -> Option<NonEmptyString> {
        as_nonempty(&self.buchungstag)
    }
    fn valuta(&self) -> Option<NonEmptyString> {
        as_nonempty(&self.valuta)
    }
    fn betrag(&self) -> Option<NonEmptyString> {
        as_nonempty(&self.betrag)
    }
    fn währung(&self) -> Option<NonEmptyString> {
        as_nonempty(&self.währung)
    }
    fn empfänger_name(&self) -> Option<NonEmptyString> {
        as_nonempty(&self.empfänger_name)
    }
    fn gläubiger_id(&self) -> Option<NonEmptyString> {
        as_nonempty(&self.gläubiger_id)
    }
    fn gegenkonto(&self) -> Option<NonEmptyString> {
        as_nonempty(&self.gegenkonto)
    }
    fn verwendungszweck(&self) -> Option<NonEmptyString> {
        as_nonempty(&self.verwendungszweck)
    }
    fn buchungstext(&self) -> Option<NonEmptyString> {
        as_nonempty(&self.buchungstext)
    }
}

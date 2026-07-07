use einlesen_domain::{NonEmptyString, Rohzeile};

/// Der rohe Datensatz **einer** Zeile, wie ihn die [`Rohquelle`](crate::Rohquelle) liefert:
/// die zehn verwerteten CSV-CAMT-v8-Felder als rohe Zellen (leer/fehlend = `None`). Er
/// erfüllt den Domänen-Port `Rohzeile`, indem er jede Zelle über `NonEmptyString` an die
/// Grenze reicht — **wie** eine CSV-Spalte einem Feld zugeordnet wird, kapselt er hier, die
/// Domäne fordert nur an.
///
/// Für Specs/io wird er über [`Rohsatz::gültig`] + die `mit_*`/`ohne_*`-Bauer bestückt; die
/// Zellen selbst bleiben gekapselt.
#[derive(Debug, Clone)]
pub struct Rohsatz {
    auftragskonto: Option<String>,
    buchungstag: Option<String>,
    valuta: Option<String>,
    betrag: Option<String>,
    währung: Option<String>,
    empfänger_name: Option<String>,
    gläubiger_id: Option<String>,
    gegenkonto: Option<String>,
    verwendungszweck: Option<String>,
    buchungstext: Option<String>,
}

impl Rohsatz {
    /// Ein vollständig gültiger Rohsatz; die `mit_*`/`ohne_*`-Bauer überschreiben einzelne
    /// Felder, um genau deren Pflicht-/Optional-Verhalten zu isolieren.
    pub fn gültig() -> Self {
        Self {
            auftragskonto: feld("DE89370400440532013000"),
            buchungstag: feld("13.06.2024"),
            valuta: feld("14.06.2024"),
            betrag: feld("-50,00"),
            währung: feld("EUR"),
            empfänger_name: feld("EDEKA SAGT DANKE"),
            gläubiger_id: feld("DE98ZZZ09999999999"),
            gegenkonto: feld("DE11520513735120710131"),
            verwendungszweck: feld("Einkauf"),
            buchungstext: feld("LASTSCHRIFT"),
        }
    }

    pub fn mit_betrag(mut self, wert: &str) -> Self {
        self.betrag = feld(wert);
        self
    }

    pub fn ohne_betrag(mut self) -> Self {
        self.betrag = None;
        self
    }

    pub fn mit_empfänger_name(mut self, wert: &str) -> Self {
        self.empfänger_name = feld(wert);
        self
    }

    pub fn mit_valuta(mut self, wert: &str) -> Self {
        self.valuta = feld(wert);
        self
    }

    pub fn ohne_gegenkonto(mut self) -> Self {
        self.gegenkonto = None;
        self
    }

    pub fn ohne_verwendungszweck(mut self) -> Self {
        self.verwendungszweck = None;
        self
    }

    pub fn ohne_buchungstext(mut self) -> Self {
        self.buchungstext = None;
        self
    }
}

/// Baut eine gesetzte Zelle.
fn feld(wert: &str) -> Option<String> {
    Some(wert.to_string())
}

/// Reicht eine rohe Zelle als `NonEmptyString` an die Domänengrenze (leer/`None` → `None`);
/// der `NonEmptyString`-Kern trimmt und weist Leeres genau hier ab.
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

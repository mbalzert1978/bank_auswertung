use auswerten_domain::{CategoryAnswer, CategoryRequest};

/// Die **Anfrage ans Orakel** als Zeile — das **Egress-DTO** dieses Slice: hier, und nur hier,
/// verlassen Daten die Maschine (ADR-0002/ADR-0004; der Bericht bekommt sein eigenes, wenn er
/// gebaut wird).
///
/// **Zwei Felder, beide unbedenklich per Konstruktion.** Der `fingerprint` ist ein **Derivat** —
/// eine öffentliche Gläubiger-ID oder ein seed-basiertes Pseudonym, nie ein Rohwert und nie der
/// Seed. Der `text` hat stromaufwärts den `ObfuskierterText`-Engpass passiert. Ein **struktureller
/// Identifikator** (IBAN, Kontonummer, BIC) hat hier keinen Platz — nicht weil er herausgefiltert
/// würde, sondern weil dieses DTO kein Feld hat, in das er passte.
///
/// **Die Sperre greift zur Compile-Zeit, nicht per Test:** dieser Typ leitet **kein**
/// `Serialize`/`Deserialize` ab, und der `assert_not_impl_any!`-Guard in `lib.rs` schlägt fehl,
/// sobald jemand es ihm gibt. Einen rohen Identifikator in etwas einzubetten, das hinausgeht, muss
/// ein **Compile-Fehler** sein, keine Review-Beobachtung — genau die Linie, die
/// `obfuskierung`s `ObfuscatedBooking` an der Datenschutz-Grenze zieht.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryRequestRow {
    /// Der Schlüssel der Buchungs*art* — und die Achse, auf der die Antwort zurückfindet.
    pub fingerprint: String,
    /// Der obfuskierte Text, an dem das Orakel die Kategorie erraten soll.
    pub text: String,
}

/// Projiziert die Domänen-Anfrage auf die Zeile — die einzige Stelle, an der sie die Adapter-Grenze
/// nach außen überquert (die `CategorizerBridge` ruft sie).
impl From<&CategoryRequest> for CategoryRequestRow {
    fn from(request: &CategoryRequest) -> Self {
        Self {
            fingerprint: request.fingerprint().as_str().to_string(),
            text: request.text().as_str().to_string(),
        }
    }
}

/// Die **Antwort des Orakels** als Zeile — das eingehende DTO der anderen Richtung (ADR-0002: je
/// Richtung ein eigenes schmales DTO, nie ein gemeinsames für Frage und Antwort).
///
/// Sie trägt den `fingerprint` als **Zuordnungsachse**, nie den Text. Der Rückweg vom Etikett auf
/// den Freitext, aus dem es erraten wurde, existiert damit nicht.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryAnswerRow {
    /// Der Schlüssel, auf den die Kategorie zugeordnet wird.
    pub fingerprint: String,
    /// Das vergebene Etikett (CONTEXT.md „Kategorie").
    pub category: String,
}

/// Eine **zugeordnete Kategorie** in der abgelegten Ergebnis-Zeile — was das Orakel zu den zuvor
/// unbekannten Buchungsarten dieses Fensters beigetragen hat.
///
/// **Bewusst ein eigener Typ**, nicht die [`CategoryAnswerRow`] wiederverwendet: die eine ist das
/// DTO eines *Ports* (was hereinkam), die andere die Zelle einer *Tabelle* (was abgelegt wird).
/// Dass sie heute dieselbe Form haben, macht sie nicht zum selben Ding — genau die Linie, die
/// dieser Slice bei `BookingDirection` und `CounterAccountRow` schon zieht.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryRow {
    pub fingerprint: String,
    pub category: String,
}

/// Projiziert die Domänen-Antwort auf die abgelegte Zelle („innen → außen").
impl From<&CategoryAnswer> for CategoryRow {
    fn from(answer: &CategoryAnswer) -> Self {
        Self {
            fingerprint: answer.fingerprint().as_str().to_string(),
            category: answer.category().as_str().to_string(),
        }
    }
}

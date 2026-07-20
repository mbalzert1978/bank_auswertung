use crate::value_objects::NonEmptyString;

/// Das **Gegenkonto einer Buchung, wie dieser Slice es sieht**: ein `KONTO_…`-Token, das
/// `obfuskieren` stromaufwärts seed-basiert vergeben hat (ADR-0010). Es ist **kein struktureller
/// Identifikator** — die rohe IBAN, aus der es entstand, bleibt vor der Datenschutz-Grenze und
/// erreicht diesen Slice zu keinem Zeitpunkt.
///
/// Opak: der Slice deutet das Token nie, er **vergleicht** es nur — gegen die mit **demselben Seed**
/// tokenisierte Eigen-Menge ([`OwnAccounts`](crate::OwnAccounts)). Genau darum ist der Vergleich
/// überhaupt tragfähig: gleicher Seed + gleiche IBAN → gleiches Token (der `Obfuscator`-Vertrag).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AccountToken(NonEmptyString);

impl AccountToken {
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl From<NonEmptyString> for AccountToken {
    fn from(value: NonEmptyString) -> Self {
        Self(value)
    }
}

/// Die **Rolle eines eigenen Kontos** — der Grund, warum die Eigen-Menge nicht bloß eine Menge von
/// Token ist: die Rolle **trägt die Deutung**. Ein Zufluss aufs Spar-Konto ist Sparen, ein Abfluss
/// aufs Haushalts-Konto eine Umbuchung, kein Konsum. Ohne sie wäre „eigen" eine Information, aus der
/// sich nichts folgern ließe.
///
/// Ein **geschlossener Summentyp** (ADR-0005): das Vokabular der Config ist endlich und hier
/// abschließend genannt. Eine Rolle außerhalb dieser vier ist damit kein Wert, der durchrutschen
/// könnte — der Adapter kann sie gar nicht erst bilden.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AccountRole {
    Giro,
    Haushalt,
    Spar,
    Vorlauf,
}

/// Wie ein Gegenkonto **fachlich einzuordnen** ist (ADR-0013, Entscheidung F): ein **eigenes** Konto
/// — dann **samt seiner Rolle** — oder ein fremdes.
///
/// Ein **Summentyp, kein Flag plus Optionalfeld** (ADR-0005): „Fremd **mit** Rolle" ist damit
/// **unkonstruierbar**, nicht „geprüft". Ebenso wenig gibt es ein „Eigen **ohne** Rolle" — die
/// Deutung ist an die Zugehörigkeit gebunden, nicht optional daneben.
///
/// Das **Netting** auf dieser Auflösung ist bewusst **nicht** hier
/// ([Issue 0004](../../../../docs/issues/0004-umbuchung-netting.md)); dieses Ticket liefert die
/// Auflösung, auf der es später aufsetzt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CounterAccount {
    Own(AccountRole),
    Foreign,
}

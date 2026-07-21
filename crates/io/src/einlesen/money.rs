//! Die echte io-Erfüllung des öffentlichen [`MoneySeam`] des `einlesen`-Adapters
//! ([Issue 0050](../../../../docs/issues/0050-money-calendar-decode-seams-real.md)): ein **Adapter**,
//! der die battle-tested [`rust_decimal`]-Lib auf unsere Seam-Signatur andockt — rein, kein I/O. Sie
//! tritt im End-to-End-Test an die Stelle des In-Memory-Fakes.
//!
//! Der Seam ist **bewusst währungs-agnostisch** bei fester Skala 2 (Cent); die währungs-bewusste
//! Minor-Unit-Deutung per ISO 4217 ist ausgelagert
//! ([Issue 0055](../../../../docs/issues/0055-waehrungs-bewusster-betrags-decode-minor-unit-per-iso-4217-von-der-domaene-hoch.md)).
//! `rust_decimal` rechnet exakt (kein `f64`-Rundungsfehler); unsere **dünne** Nutzungslogik streift
//! das führende Vorzeichen ab und bringt die deutsche Gruppierung/Dezimalstelle in die Form, die die
//! Lib parst. Ein undeutbarer Wert ergibt `None`, was das Aggregat terminal auf `AmountInvalid` mappt.

use core::str::FromStr;

use einlesen_adapter::{DecodedAmount, MoneySeam, Sign};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;

/// Der echte Geld-Seam: dockt `rust_decimal` auf [`MoneySeam`] an. **Zustandslos** — ein Wert genügt.
pub struct GermanMoney;

impl MoneySeam for GermanMoney {
    fn decode(&self, raw: &str) -> Option<DecodedAmount> {
        // Das führende Vorzeichen ist **Pflicht** (Seam-Vertrag: ohne deutbares Vorzeichen `None`,
        // nie still-positiv) und trennt den Ziffernkörper ab.
        let (sign, body) = raw
            .strip_prefix('-')
            .map(|rest| (Sign::Negative, rest))
            .or_else(|| raw.strip_prefix('+').map(|rest| (Sign::Positive, rest)))?;

        // Ganzzahl- und (optionalen) Nachkomma-Teil am Dezimalkomma trennen.
        let (integer_part, fraction) = match body.split_once(',') {
            Some((integer_part, fraction)) => (integer_part, Some(fraction)),
            None => (body, None),
        };

        // Deutsche Darstellung → die Form, die `rust_decimal` parst — **mit** Gruppierungs-Prüfung:
        // ein Tausenderpunkt zählt nur, wenn er sauber in 3er-Gruppen trennt; eine falsch gruppierte
        // Zahl (z. B. „1.23") ergibt `None` statt einer stillen Fehldeutung („123").
        let integer_digits = normalise_grouped_integer(integer_part)?;
        let normalised = match fraction {
            None => integer_digits,
            Some(fraction) => format!("{integer_digits}.{}", validated_fraction(fraction)?),
        };

        // `rust_decimal` parst exakt (kein `f64`-Rundungsfehler). Der Nachkomma-Teil ist auf ≤2
        // Stellen begrenzt (feste Skala 2 = Cent), darum geht `× 100` immer in ganzen Cent auf.
        let cents = Decimal::from_str(&normalised)
            .ok()?
            .checked_mul(Decimal::ONE_HUNDRED)?;
        Some(DecodedAmount {
            cent: cents.to_u64()?,
            sign,
        })
    }
}

/// Ist `s` ein **nichtleerer** reiner ASCII-Ziffernblock?
fn all_digits(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|byte| byte.is_ascii_digit())
}

/// Prüft die deutsche Tausender-Gruppierung des Ganzzahl-Teils und gibt den reinen Ziffernkörper
/// (Punkte entfernt) zurück, sonst `None`. **Ohne** Punkt ist jeder nichtleere Ziffernblock gültig;
/// **mit** Punkt hat der erste Block 1–3 Ziffern und jeder weitere **genau** 3 — so wird „1.23" oder
/// „1.2345" verworfen statt fehlgedeutet.
fn normalise_grouped_integer(integer_part: &str) -> Option<String> {
    let groups: Vec<&str> = integer_part.split('.').collect();
    let valid = match groups.as_slice() {
        [single] => all_digits(single),
        [first, rest @ ..] => {
            (1..=3).contains(&first.len())
                && all_digits(first)
                && rest
                    .iter()
                    .all(|group| group.len() == 3 && all_digits(group))
        }
        [] => false,
    };
    valid.then(|| groups.concat())
}

/// Prüft den Nachkomma-Teil: **1–2** Ziffern, sonst `None` (ein leerer Teil = nachgestelltes Komma
/// wie „50," ist ungültig; drei+ Stellen sprengen die Cent-Skala).
fn validated_fraction(fraction: &str) -> Option<&str> {
    ((1..=2).contains(&fraction.len()) && all_digits(fraction)).then_some(fraction)
}

use domain::{Betrag, Buchungsart, Richtung, Währung};

use crate::fehler::ImportError;

/// Mappt einen **bereits normalisierten** Währungscode (getrimmt,
/// Großbuchstaben — siehe [`währung_normalized`]) auf die Domänen-`Währung`.
/// Nur `EUR` ist bekannt; alles andere bricht laut ab (die Domäne summiert keine
/// fremde Währung).
pub(crate) fn währung_from_str(normalized: &str) -> Result<Währung, ImportError> {
    match normalized {
        "EUR" => Ok(Währung::Eur),
        other => Err(ImportError::UnknownWährung(other.to_string())),
    }
}

/// Die eine Stelle, an der „was ist derselbe Währungscode" entschieden wird:
/// getrimmt und in Großbuchstaben. Sowohl die Einheitlichkeitsprüfung als auch
/// das Mapping auf `Währung` gehen hierdurch, damit sie nicht auseinanderlaufen.
pub(crate) fn währung_normalized(raw: &str) -> String {
    raw.trim().to_uppercase()
}

/// Parst einen deutschen Geldbetrag (`-1.234,56`) in `Richtung` + vorzeichenlosen
/// `Betrag`: das Vorzeichen wird zur `Richtung` (Minus → `Ausgang`), der Rest zum
/// Betrag. So ist ein negativer `Eingang` schon an der Grenze unkonstruierbar.
pub(crate) fn betrag_parse(
    raw: &str,
    currency: Währung,
) -> Result<(Richtung, Betrag), ImportError> {
    let trimmed = raw.trim();
    let (direction, digits) = match trimmed.strip_prefix('-') {
        Some(rest) => (Richtung::Ausgang, rest),
        None => (
            Richtung::Eingang,
            trimmed.strip_prefix('+').unwrap_or(trimmed),
        ),
    };
    let cent =
        cent_from_german(digits).ok_or_else(|| ImportError::BetragInvalid(raw.to_string()))?;
    // `cent` ist konstruktionsbedingt ≥ 0 (das Vorzeichen wurde oben zur
    // `Richtung` abgetrennt), daher liefert `Betrag::new` hier praktisch immer
    // `Some`; das `ok_or_else` hält nur den Typ-Kontrakt der Domäne ehrlich.
    let amount =
        Betrag::new(cent, currency).ok_or_else(|| ImportError::BetragInvalid(raw.to_string()))?;
    Ok((direction, amount))
}

/// Wandelt den vorzeichenlosen Ziffernteil eines deutschen Betrags in Cent. Das
/// Komma trennt bis zu zwei Nachkommastellen; Tausenderpunkte müssen **korrekt
/// gruppiert** sein. `None` (→ lauter Fehler) bei allem Anderen — insbesondere
/// bei einem Punkt als Dezimaltrenner (`1234.56`) oder Streu-/Mehrfachpunkten,
/// die sonst still um Faktor 100 falsch verrechnet würden.
fn cent_from_german(digits: &str) -> Option<i64> {
    let (whole_raw, fraction) = match digits.split_once(',') {
        // Komma vorhanden, aber ohne Nachkommastellen (`12,`) ist kaputt — lieber
        // lautes Scheitern als raten.
        Some((_, "")) => return None,
        Some((whole, fraction)) => (whole, fraction),
        None => (digits, ""),
    };
    if fraction.len() > 2 || !fraction.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let whole = check_thousands_grouping(whole_raw)?;
    let whole_cent = whole.parse::<i64>().ok()?.checked_mul(100)?;
    let fraction_cent = match fraction.len() {
        1 => fraction.parse::<i64>().ok()?.checked_mul(10)?,
        2 => fraction.parse::<i64>().ok()?,
        _ => 0,
    };
    whole_cent.checked_add(fraction_cent)
}

/// Prüft den Ganzzahlteil und liefert ihn ohne Tausenderpunkte zurück. Erlaubt
/// reine Ziffern (`1234`) oder in exakte 3er-Gruppen gepunktete Zahlen (`1.234`,
/// `12.345.678`); lehnt Fehlgruppierung, Streu-/Randpunkte (`1234.56`, `1..2`,
/// `.5`, `5.`) und führende Nullen (`007`, `01.234` — außer der echten `0`)
/// mit `None` ab.
fn check_thousands_grouping(whole: &str) -> Option<String> {
    if !whole.contains('.') {
        let only_digits = !whole.is_empty() && whole.chars().all(|c| c.is_ascii_digit());
        // Führende Null nur bei der echten Null selbst zulässig — konsistent mit
        // dem gepunkteten Pfad unten, der `01.234` ablehnt.
        let no_leading_zero = whole == "0" || !whole.starts_with('0');
        return (only_digits && no_leading_zero).then(|| whole.to_string());
    }
    let groups: Vec<&str> = whole.split('.').collect();
    let (first, rest) = groups.split_first()?;
    let first_ok = (1..=3).contains(&first.len())
        && !first.starts_with('0')
        && first.chars().all(|c| c.is_ascii_digit());
    let rest_ok = !rest.is_empty()
        && rest
            .iter()
            .all(|group| group.len() == 3 && group.chars().all(|c| c.is_ascii_digit()));
    (first_ok && rest_ok).then(|| groups.concat())
}

/// Deutet den Buchungstext (die maschinelle Art) auf die `Buchungsart`. Erkennt
/// die bekannten Arten per Teilzeichenkette (case-insensitiv); alles Übrige ist
/// `Sonstige`.
pub(crate) fn buchungsart_from_text(text: &str) -> Buchungsart {
    const PATTERNS: &[(&str, Buchungsart)] = &[
        ("LASTSCHRIFT", Buchungsart::Lastschrift),
        ("DAUERAUFTRAG", Buchungsart::Dauerauftrag),
        ("ÜBERWEISUNG", Buchungsart::Überweisung),
        ("UEBERWEISUNG", Buchungsart::Überweisung),
        ("ENTGELT", Buchungsart::Entgelt),
    ];
    let upper = text.to_uppercase();
    PATTERNS
        .iter()
        .find(|(pattern, _)| upper.contains(pattern))
        .map_or(Buchungsart::Sonstige, |(_, kind)| *kind)
}

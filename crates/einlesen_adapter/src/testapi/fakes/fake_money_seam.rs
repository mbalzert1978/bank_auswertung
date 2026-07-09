use crate::abstractions::{DecodedAmount, MoneySeam, Sign};

/// Deterministischer Fake: dekodiert das **deutsche** Betragsfeld („-1.234,56") zu seinen
/// Bestandteilen; ungültiges Format ergibt `None`. Implementiert den öffentlichen
/// [`MoneySeam`] (nicht den Domänen-Port) — hinter dem echten Seam steckt später io
/// (Vorbild `FakeSftpPuller : ISftpPuller`). Die Format-Regeln leben **hier** (hinter dem
/// Seam), nicht in der Domäne (ADR-0008): Tausenderpunkt, Dezimalkomma mit genau zwei
/// Nachkommastellen, Gruppierungs-Prüfung und das drei-wertige führende Vorzeichen — ein
/// fehlendes wird als [`Sign::Missing`] gemeldet, nie still positiv.
pub(crate) struct FakeMoneySeam;

impl MoneySeam for FakeMoneySeam {
    fn decode(&self, raw: &str) -> Option<DecodedAmount> {
        let (sign, digits) = split_sign(raw);
        let cent = cent_from_german_digits(digits)?;
        Some(DecodedAmount { cent, sign })
    }
}

/// Erkennt das führende Vorzeichen und streift es ab — **drei-wertig**: fehlt es, bleiben
/// die Ziffern unverändert und das Vorzeichen ist `Missing` (kein stilles Positiv; die
/// Domäne macht daraus `SignMissing`).
fn split_sign(field: &str) -> (Sign, &str) {
    field
        .strip_prefix('-')
        .map(|digits| (Sign::Negative, digits))
        .or_else(|| {
            field
                .strip_prefix('+')
                .map(|digits| (Sign::Positive, digits))
        })
        .unwrap_or((Sign::Missing, field))
}

/// Dekodiert die **vorzeichenlosen** deutschen Betragsziffern („1.234,56") zur Cent-Menge:
/// das Komma trennt genau zwei Nachkommastellen (fehlt es, sind es null); Tausenderpunkte
/// müssen echte Gruppierung sein. Ungültiges ergibt `None`.
fn cent_from_german_digits(digits: &str) -> Option<u64> {
    let (euro_part, cent_part) = match digits.split_once(',') {
        Some((euro, cent)) => (euro, Some(cent)),
        None => (digits, None),
    };
    if !grouping_is_valid(euro_part) {
        return None;
    }
    let euro: u64 = euro_part.replace('.', "").parse().ok()?;
    let cent: u64 = match cent_part {
        None => 0,
        Some(decimals) if decimals.len() == 2 && is_digits(decimals) => decimals.parse().ok()?,
        Some(_) => return None,
    };
    euro.checked_mul(100)?.checked_add(cent)
}

/// Gültige Gruppierung des Euro-Teils: ohne Punkt nur Ziffern; mit Punkten trägt die
/// erste Gruppe 1–3 Ziffern, jede weitere genau 3.
fn grouping_is_valid(euro_part: &str) -> bool {
    match euro_part.split_once('.') {
        None => !euro_part.is_empty() && is_digits(euro_part),
        Some((first, rest)) => {
            (1..=3).contains(&first.len())
                && is_digits(first)
                && rest
                    .split('.')
                    .all(|group| group.len() == 3 && is_digits(group))
        }
    }
}

fn is_digits(part: &str) -> bool {
    part.bytes().all(|byte| byte.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::FakeMoneySeam;
    use crate::abstractions::{DecodedAmount, MoneySeam, Sign};

    fn dekodiere(roh: &str) -> Option<DecodedAmount> {
        FakeMoneySeam.decode(roh)
    }

    #[test]
    fn negativer_betrag_mit_tausenderpunkt_wird_zu_cent_und_negativem_vorzeichen() {
        assert_eq!(
            dekodiere("-1.234,56"),
            Some(DecodedAmount {
                cent: 123_456,
                sign: Sign::Negative
            })
        );
    }

    #[test]
    fn positiver_betrag_trägt_positives_vorzeichen() {
        assert_eq!(
            dekodiere("+50,00"),
            Some(DecodedAmount {
                cent: 5_000,
                sign: Sign::Positive
            })
        );
    }

    #[test]
    fn fehlendes_vorzeichen_wird_als_missing_gemeldet_nicht_positiv() {
        assert_eq!(
            dekodiere("50,00"),
            Some(DecodedAmount {
                cent: 5_000,
                sign: Sign::Missing
            })
        );
    }

    #[test]
    fn betrag_ohne_nachkommastellen_ist_gültig() {
        assert_eq!(
            dekodiere("-1000"),
            Some(DecodedAmount {
                cent: 100_000,
                sign: Sign::Negative
            })
        );
    }

    #[test]
    fn betrag_mit_drei_nachkommastellen_ist_ungültig() {
        assert_eq!(dekodiere("-1,234"), None);
    }

    #[test]
    fn nicht_numerischer_betrag_ist_ungültig() {
        assert_eq!(dekodiere("-abc"), None);
    }

    #[test]
    fn ungültige_gruppierung_ist_ungültig() {
        // Tausenderpunkte müssen echte Dreiergruppen trennen — sonst ist der Betrag
        // mehrdeutig (Spaltenverschub/Tippfehler) und wird abgewiesen.
        assert_eq!(dekodiere("-1.23,45"), None);
        assert_eq!(dekodiere("-12.3456"), None);
        assert_eq!(dekodiere("-1234.567"), None);
        assert_eq!(dekodiere("-.500"), None);
    }
}

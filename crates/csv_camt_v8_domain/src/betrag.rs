use crate::richtung::RichtungEntity;

/// Die Währung eines `BetragEntity`. Die Domäne summiert keine gemischten
/// Währungen — der Adapter sichert eine einheitliche Währung über die Datei zu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WährungEntity {
    Eur,
}

/// Ein vorzeichenloser Geldbetrag mit Währung. Das Vorzeichen lebt
/// ausschließlich in [`RichtungEntity`], nie hier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BetragEntity {
    cent: u64,
    währung: WährungEntity,
}

impl BetragEntity {
    /// Liefert `None` bei negativem `cent` — ein negativer Betrag ist nicht
    /// konstruierbar (das Vorzeichen gehört in die `RichtungEntity`).
    pub fn new(cent: i64, währung: WährungEntity) -> Option<Self> {
        u64::try_from(cent).ok().map(|cent| Self { cent, währung })
    }

    /// Parst einen deutschen Geldbetrag (`-1.234,56`) in [`RichtungEntity`] +
    /// vorzeichenlosen `BetragEntity`: das Vorzeichen wird zur Richtung (Minus →
    /// `Ausgang`), der Rest zum Betrag. So ist ein negativer `Eingang` schon an
    /// der Konstruktion unmöglich (parse, don't validate). `None` bei allem, was
    /// kein gültiger deutscher Betrag ist — die eine Stelle, die dieses Format
    /// deutet.
    pub fn aus_deutschem_betrag(
        raw: &str,
        währung: WährungEntity,
    ) -> Option<(RichtungEntity, Self)> {
        let (richtung, digits) = RichtungEntity::aus_vorzeichen(raw.trim());
        let cent = cent_aus_deutsch(digits)?;
        let betrag = Self::new(cent, währung)?;
        Some((richtung, betrag))
    }

    pub fn cent(&self) -> u64 {
        self.cent
    }

    pub fn währung(&self) -> WährungEntity {
        self.währung
    }
}

/// Wandelt den vorzeichenlosen Ziffernteil eines deutschen Betrags in Cent. Das
/// Komma trennt bis zu zwei Nachkommastellen; Tausenderpunkte müssen **korrekt
/// gruppiert** sein. `None` bei allem Anderen — insbesondere bei einem Punkt als
/// Dezimaltrenner (`1234.56`) oder Streu-/Mehrfachpunkten, die sonst still um
/// Faktor 100 falsch verrechnet würden.
fn cent_aus_deutsch(digits: &str) -> Option<i64> {
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
    let whole = tausender_gruppierung_geprüft(whole_raw)?;
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
/// `.5`, `5.`) und führende Nullen (`007`, `01.234` — außer der echten `0`) mit
/// `None` ab.
fn tausender_gruppierung_geprüft(whole: &str) -> Option<String> {
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

#[cfg(test)]
mod tests {
    use super::{BetragEntity, WährungEntity};
    use crate::richtung::RichtungEntity;

    fn eur(cent: i64) -> BetragEntity {
        BetragEntity::new(cent, WährungEntity::Eur).expect("nicht-negativer Testbetrag")
    }

    #[test]
    fn plus_und_kein_vorzeichen_werden_zu_eingang() {
        assert_eq!(
            BetragEntity::aus_deutschem_betrag("+5,00", WährungEntity::Eur),
            Some((RichtungEntity::Eingang, eur(500)))
        );
        assert_eq!(
            BetragEntity::aus_deutschem_betrag("1234", WährungEntity::Eur),
            Some((RichtungEntity::Eingang, eur(123_400)))
        );
    }

    #[test]
    fn minus_wird_zu_ausgang_mit_tausenderpunkt_und_einer_nachkommastelle() {
        assert_eq!(
            BetragEntity::aus_deutschem_betrag("-1.234,5", WährungEntity::Eur),
            Some((RichtungEntity::Ausgang, eur(123_450)))
        );
    }

    #[test]
    fn punkt_als_dezimaltrenner_ist_kein_betrag() {
        // `1234.56` würde sonst still um Faktor 100 falsch verrechnet.
        assert_eq!(
            BetragEntity::aus_deutschem_betrag("1234.56", WährungEntity::Eur),
            None
        );
    }

    #[test]
    fn falsch_gruppierte_tausenderpunkte_sind_kein_betrag() {
        assert_eq!(
            BetragEntity::aus_deutschem_betrag("1..2,00", WährungEntity::Eur),
            None
        );
    }

    #[test]
    fn komma_ohne_nachkommastellen_ist_kein_betrag() {
        assert_eq!(
            BetragEntity::aus_deutschem_betrag("12,", WährungEntity::Eur),
            None
        );
    }

    #[test]
    fn führende_null_in_tausendergruppe_ist_kein_betrag() {
        assert_eq!(
            BetragEntity::aus_deutschem_betrag("01.234,56", WährungEntity::Eur),
            None
        );
    }

    #[test]
    fn führende_null_ohne_gruppierung_ist_kein_betrag() {
        // Die echte `0` bleibt zulässig, `007,50` nicht.
        assert_eq!(
            BetragEntity::aus_deutschem_betrag("007,50", WährungEntity::Eur),
            None
        );
    }

    #[test]
    fn nur_vorzeichen_ohne_ziffern_ist_kein_betrag() {
        assert_eq!(
            BetragEntity::aus_deutschem_betrag("-", WährungEntity::Eur),
            None
        );
    }

    #[test]
    fn nicht_numerische_nachkommastellen_sind_kein_betrag() {
        assert_eq!(
            BetragEntity::aus_deutschem_betrag("12,ab", WährungEntity::Eur),
            None
        );
    }

    #[test]
    fn negativer_cent_ist_nicht_konstruierbar() {
        assert_eq!(BetragEntity::new(-1, WährungEntity::Eur), None);
    }
}

//! Reine Invariant-Absicherung, kein Domänenkonzept: strukturelle
//! Identifikatoren (`Iban`/`Bic`) gehören nach ADR-0001/0003 in die
//! adapterseitige Randschicht, nie in die Domäne. Die echte, prüfsummen-
//! validierende `Iban` lebt inzwischen im `eigene_konten`-Crate (leitet dort
//! ebenfalls **kein** `Serialize` ab); der CSV-CAMT-v8-Adapter (`csv_camt_v8`)
//! hält rohe IBAN/BIC beim Mappen zurück. `Bic` ist bis auf Weiteres reiner
//! Platzhalter — noch braucht ihn kein Format.
//!
//! Diese Platzhalter-Typen existieren nur, damit der Datenschutz-Invariant —
//! kein `Serialize` ableitbar, in keinem DTO einbettbar — hier in der Domäne
//! compilergeprüft festgenagelt bleibt (strikt stärker als eine Laufzeitprüfung).

#[expect(
    dead_code,
    reason = "Platzhalter nur für den Serialize-Invarianten-Test; bewusst nie konstruiert"
)]
pub(crate) struct Iban(String);
#[expect(
    dead_code,
    reason = "Platzhalter nur für den Serialize-Invarianten-Test; bewusst nie konstruiert"
)]
pub(crate) struct Bic(String);

#[cfg(test)]
mod tests {
    use super::{Bic, Iban};
    use serde::Serialize;
    use static_assertions::assert_not_impl_any;

    // Tragender Invariant-Test: ein Versuch, Iban/Bic serialisierbar zu
    // machen (und damit in ein DTO einzubetten), ist ein Compile-Fehler.
    assert_not_impl_any!(Iban: Serialize);
    assert_not_impl_any!(Bic: Serialize);
}

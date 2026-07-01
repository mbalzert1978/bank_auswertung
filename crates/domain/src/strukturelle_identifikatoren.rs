//! Reine Invariant-Absicherung, kein Domänenkonzept: `Iban`/`Bic` leben nach
//! ADR-0001/0003 ausschließlich im eingehenden CSV-Adapter (Baureihenfolge-
//! Schritt 2), der noch nicht existiert. Diese Platzhalter-Typen existieren
//! nur, damit der Datenschutz-Invariant — kein `Serialize` ableitbar, in
//! keinem DTO einbettbar — schon jetzt compilergeprüft ist, nicht erst wenn
//! der echte Adapter-Crate entsteht.

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

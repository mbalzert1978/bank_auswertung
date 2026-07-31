use obfuskierung_domain::{AccountToken, NonEmptyString, Obfuscator, Seed};

use crate::abstractions::ObfuscatorSeam;
use crate::adapter::ObfuscatorBridge;

/// Formt das **Konto-Token**, das dieser Slice einer rohen IBAN vergäbe — dieselbe eine Ableitung,
/// über die eine verarbeitete Buchung ihr Auftrags- und Gegenkonto tokenisiert.
///
/// **Warum das öffentlich ist.** Die Eigene-Konten-Config des `auswerten`-Slice trägt **Token, keine
/// IBANs** (ADR-0013, Entscheidung F): ihr Loader in `io` muss die eigenen IBANs mit **demselben
/// Obfuskator und demselben Seed** tokenisieren, mit denen `obfuskieren` die Gegenkonto-Token gebildet
/// hat — sonst verglichen `auswerten`s Token nachher zwei verschiedene Alphabete, und **jede**
/// Umbuchung liefe lautlos als *fremd* durch. Wäre die Ableitung von außen nicht zu haben, müsste `io`
/// das `KONTO_`-Präfix nachbauen; genau diese Doppelung schließt diese Funktion aus. Der eine Ort, an
/// dem ein Konto-Token **entsteht**, bleibt die Domäne — von hier aus ist er nur **erreichbar**.
///
/// **Kein Leck:** herein geht ein lokal erlaubter Klartext, heraus kommt allein das Token; der Seed
/// bleibt beim Aufrufer, und die IBAN steckt im Ergebnis nicht mehr (das ist die Zusage des
/// [`ObfuscatorSeam`]). Die Richtung ist damit dieselbe wie im Slice: roh hinein, Token hinaus, nie
/// zurück.
///
/// `None`, wenn der Klartext nach dem Trimmen leer ist — ein leeres Konto ist keins, und ein Token
/// darüber wäre ein Token über nichts.
#[must_use]
pub fn account_token(seed: u64, iban: &str, obfuscator: &dyn ObfuscatorSeam) -> Option<String> {
    let plaintext = NonEmptyString::new(iban)?;
    let replacement = ObfuscatorBridge { seam: obfuscator }.obfuscate(Seed::new(seed), &plaintext);
    Some(
        AccountToken::from_replacement(replacement)
            .as_str()
            .to_string(),
    )
}

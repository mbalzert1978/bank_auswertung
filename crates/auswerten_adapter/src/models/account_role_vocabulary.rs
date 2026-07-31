use auswerten_domain::AccountRole;

/// Das **Vokabular der Konto-Rollen an der Adapter-Grenze** — die eine Stelle, an der die vier
/// Domänen-Varianten ihre Zeichenketten-Form bekommen.
///
/// **Warum hier und nicht zweimal.** Die Rolle überquert die Grenze in **beide** Richtungen: herein
/// aus der Eigene-Konten-Config ([`OwnAccountsConfig`](crate::OwnAccountsConfig)), hinaus in die
/// abgelegte Ergebnis-Zeile ([`CounterAccountRow`](crate::CounterAccountRow)). Stünde das Vokabular
/// zweimal da, könnten die Richtungen auseinanderlaufen — eine Config, die `vorlauf` liest, und ein
/// Ergebnis, das `Vorlauf` schreibt, wären beide „richtig" und trotzdem nicht mehr dasselbe Wort.
/// Hier ist der Rundlauf per Konstruktion dicht: **das Parsen benutzt die Beschriftung**.
///
/// Die Rolle „innen → außen". Ein **erschöpfender `match`**, bewusst keine Tabelle: so erzwingt der
/// Compiler die Beschriftung einer fünften Variante, sollte je eine dazukommen.
pub(crate) fn role_label(role: AccountRole) -> &'static str {
    match role {
        AccountRole::Giro => "giro",
        AccountRole::Haushalt => "haushalt",
        AccountRole::Spar => "spar",
        AccountRole::Vorlauf => "vorlauf",
    }
}

/// Alle Rollen, über die das Parsen sucht. Die **Wörter** stehen nicht hier — nur die Varianten;
/// beschriftet werden sie von [`role_label`]. Fehlte hier je eine neu hinzugekommene Variante, wäre
/// die Folge eine **Ablehnung** der Config (laut und terminal), kein still falsches Etikett im
/// abgelegten Ergebnis — die harmlosere der beiden Driftrichtungen.
const ALL: [AccountRole; 4] = [
    AccountRole::Giro,
    AccountRole::Haushalt,
    AccountRole::Spar,
    AccountRole::Vorlauf,
];

/// Deutet die rohe Rollen-Zelle „außen → innen". Getrimmt und groß-/kleinschreibungs-tolerant — das
/// ist **Mechanik** am Rand (wie das Trimmen des `NonEmptyString`), keine fachliche Entscheidung;
/// welche vier Rollen es gibt, sagt die Domäne.
///
/// **Allokationsfrei:** verglichen wird über `eq_ignore_ascii_case` gegen das ASCII-Vokabular, statt
/// die Eingabe erst kleinzuschreiben. Eine Zelle mit Nicht-ASCII matcht dabei schlicht keinen
/// Eintrag — und „keine bekannte Rolle" ist genau die richtige Antwort darauf.
pub(crate) fn parse_role(raw: &str) -> Option<AccountRole> {
    let raw = raw.trim();
    ALL.into_iter()
        .find(|&role| role_label(role).eq_ignore_ascii_case(raw))
}

/// Deutet eine rohe Rollen-Zelle und gibt ihre **kanonische Beschriftung** zurück — dieselbe
/// Deutung wie das crate-interne `parse_role`, nur ohne den Domänen-Typ mitzugeben.
///
/// **Wofür.** Der Eigene-Konten-Loader in `io` schreibt die Rolle als Zeichenkette in die
/// [`OwnAccountsConfig`](crate::OwnAccountsConfig) und soll eine **unbekannte** Rolle schon beim Laden
/// laut ablehnen, statt sie bis in den Lauf zu tragen. Ohne diese Funktion müsste `io` das Vokabular
/// nachbauen — und genau dann könnten die beiden Listen auseinanderlaufen, was dieses Modul
/// ausschließt (siehe die Modul-Doku zu `role_label`). `None` heißt: keine der vier Rollen.
///
/// Zurückgegeben wird die **kanonische** Form (`giro` | `haushalt` | `spar` | `vorlauf`), nicht die
/// Eingabe: was der Loader ablegt, ist damit per Konstruktion dasselbe Wort, das die Deutung später
/// wiedererkennt.
#[must_use]
pub fn account_role_label(raw: &str) -> Option<&'static str> {
    parse_role(raw).map(role_label)
}

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

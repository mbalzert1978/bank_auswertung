//! Das **HTML** des Berichts: eine eigenständige Seite — CSS inline, keine externe Ressource, kein
//! JavaScript. Kein Template-Crate: die Seite ist eine feste Folge von Abschnitten, je einer pro
//! Zeilentyp, und **jeder** Text geht durch [`escape`] — auch die Kategorie-Etiketten, die aus einer
//! Chat-Antwort stammen (Issue 0011) und nur auf Länge und Zeichenklasse geprüft sind, nicht auf HTML.
//!
//! **Spalten und Zellen passen per Typ zusammen:** jeder Zeilentyp nennt seine `N` Spalten und gibt
//! genau `N` Zellen her ([`TableRow`]). Eine Zelle zu viel oder zu wenig ist ein **Compile-Fehler**,
//! kein still abgeschnittener Tabelleninhalt.

use crate::report::model::{
    Bericht, CashflowLine, FeeLine, RankLine, RecurrenceLine, SavingsLine, TransferLine,
};

/// Eine Tabellenspalte: Überschrift und ob sie Zahlen trägt (rechtsbündig, ohne Umbruch).
struct Column {
    title: &'static str,
    numeric: bool,
}

const fn text(title: &'static str) -> Column {
    Column {
        title,
        numeric: false,
    }
}

const fn number(title: &'static str) -> Column {
    Column {
        title,
        numeric: true,
    }
}

/// Ein Zeilentyp des Berichts als Abschnitt: Anker, Überschrift, ein Satz Erklärung, die `N` Spalten
/// und je Zeile genau `N` Zellen.
trait TableRow<const N: usize> {
    const ID: &'static str;
    const TITLE: &'static str;
    const INTRO: &'static str;
    const COLUMNS: [Column; N];

    fn cells(&self) -> [&str; N];
}

impl TableRow<4> for CashflowLine {
    const ID: &'static str = "cashflow";
    const TITLE: &'static str = "Cashflow je Konto und Monat";
    const INTRO: &'static str = "Was auf jedem eigenen Konto ein- und abging — Umbuchungen zwischen \
        den eigenen Konten sind herausgerechnet, Gebühren sind enthalten.";
    const COLUMNS: [Column; 4] = [
        text("Konto"),
        text("Monat"),
        number("Eingang"),
        number("Ausgang"),
    ];

    fn cells(&self) -> [&str; 4] {
        [&self.account, &self.month, &self.incoming, &self.outgoing]
    }
}

impl TableRow<4> for TransferLine {
    const ID: &'static str = "umbuchungen";
    const TITLE: &'static str = "Umbuchungen zwischen eigenen Konten";
    const INTRO: &'static str = "Was aus dem Cashflow herausgerechnet wurde: ein Ausgang ging auf \
        das genannte eigene Konto, ein Eingang kam von dort.";
    const COLUMNS: [Column; 4] = [
        text("Monat"),
        text("Gegenkonto"),
        number("Eingang"),
        number("Ausgang"),
    ];

    fn cells(&self) -> [&str; 4] {
        [
            &self.month,
            &self.counter_account,
            &self.incoming,
            &self.outgoing,
        ]
    }
}

impl TableRow<4> for FeeLine {
    const ID: &'static str = "gebuehren";
    const TITLE: &'static str = "Gebühren";
    const INTRO: &'static str = "Was die Bank selbst eingezogen hat — echter Abfluss, darum auch im \
        Cashflow enthalten.";
    const COLUMNS: [Column; 4] = [
        text("Monat"),
        text("Konto"),
        number("Eingezogen"),
        number("Erstattet"),
    ];

    fn cells(&self) -> [&str; 4] {
        [&self.month, &self.account, &self.charged, &self.refunded]
    }
}

impl TableRow<9> for RankLine {
    const ID: &'static str = "ranking";
    const TITLE: &'static str = "Pareto-Ranking der Kostentreiber";
    const INTRO: &'static str = "Die größten Abflüsse zuerst. Der kumulierte Anteil zeigt, wie wenige \
        Treiber den Großteil ausmachen.";
    const COLUMNS: [Column; 9] = [
        text("Treiber"),
        text("Kategorie"),
        number("Abfluss"),
        number("Buchungen"),
        number("Anteil"),
        number("Kumuliert"),
        number("Jahresbetrag"),
        text("Takt"),
        number("Platz im Ranking"),
    ];

    fn cells(&self) -> [&str; 9] {
        [
            &self.driver,
            &self.category,
            &self.outflow,
            &self.bookings,
            &self.share,
            &self.cumulative_share,
            &self.annual,
            &self.rhythm,
            &self.rank,
        ]
    }
}

impl TableRow<7> for SavingsLine {
    const ID: &'static str = "spar-kandidaten";
    const TITLE: &'static str = "Spar-Kandidaten";
    const INTRO: &'static str = "Treiber, deren Jahresbetrag die Schwelle aus der Spar-Config \
        erreicht — ein Vorschlag, entschieden wird von dir.";
    const COLUMNS: [Column; 7] = [
        text("Treiber"),
        text("Kategorie"),
        number("Jahresbetrag"),
        number("Anteil"),
        text("Takt"),
        text("Preissteigerung"),
        number("Platz im Ranking"),
    ];

    fn cells(&self) -> [&str; 7] {
        [
            &self.driver,
            &self.category,
            &self.annual,
            &self.share,
            &self.rhythm,
            &self.price_increase,
            &self.rank,
        ]
    }
}

impl TableRow<7> for RecurrenceLine {
    const ID: &'static str = "wiederkehr";
    const TITLE: &'static str = "Wiederkehrende Abflüsse";
    const INTRO: &'static str = "Abos, Verträge, Daueraufträge: monatlich oder jährlich, der höchste \
        Jahresbetrag zum aktuellen Preis zuerst. Händler hinter einem Zahlungsdienst stehen einzeln.";
    const COLUMNS: [Column; 7] = [
        text("Treiber"),
        text("Kategorie"),
        text("Takt"),
        number("Je Abbuchung"),
        number("Jahresbetrag"),
        text("Preissteigerung"),
        text("Konto"),
    ];

    fn cells(&self) -> [&str; 7] {
        [
            &self.driver,
            &self.category,
            &self.rhythm,
            &self.typical,
            &self.annual,
            &self.price_increase,
            &self.account,
        ]
    }
}

impl Bericht {
    /// Rendert den Bericht als vollständige HTML-Seite: Kopf, Hinweis, Navigation, dann je Zeilentyp
    /// ein Abschnitt.
    pub(crate) fn render(&self) -> String {
        [
            HEAD.to_string(),
            format!(
                "<header><h1>Bank-Auswertung</h1><p class=\"lead\">Wohin das Geld fließt — \
                 errechnet aus den eingelesenen Kontoauszügen. Währung: {}.</p></header>\n",
                escape(&self.currency)
            ),
            self.notice(),
            NAV.to_string(),
            section(&self.cashflows),
            section(&self.transfers),
            section(&self.fees),
            section(&self.ranking),
            section(&self.savings),
            section(&self.recurrences),
            FOOT.to_string(),
        ]
        .concat()
    }

    /// Der Hinweis oben im Bericht, wenn der Analyse Buchungen fehlen (ADR-0017) — sonst nichts.
    fn notice(&self) -> String {
        let missing = match self.missing_bookings {
            0 => return String::new(),
            1 => "1 Buchung fehlt".to_string(),
            many => format!("{many} Buchungen fehlen"),
        };
        format!(
            "<p class=\"warnung\" role=\"alert\"><strong>Analyse unvollständig:</strong> {missing}. \
             Sie wurden beim Verarbeiten ausgesondert (siehe Lauf-Protokoll) und sind in keiner Zahl \
             dieses Berichts enthalten.</p>\n"
        )
    }
}

/// Ein Abschnitt aus Überschrift, Erklärung und der Tabelle seiner Zeilen — ohne Zeilen steht statt
/// der Tabelle ein Satz. Jede Zelle wird escaped.
fn section<const N: usize, R: TableRow<N>>(rows: &[R]) -> String {
    let body = rows
        .iter()
        .map(|row| {
            let cells = R::COLUMNS
                .iter()
                .zip(row.cells())
                .map(|(column, cell)| format!("<td{}>{}</td>", class(column), escape(cell)))
                .collect::<String>();
            format!("<tr>{cells}</tr>\n")
        })
        .collect::<String>();
    let content = if body.is_empty() {
        "<p class=\"leer\">Keine Einträge.</p>\n".to_string()
    } else {
        let head = R::COLUMNS
            .iter()
            .map(|column| format!("<th{}>{}</th>", class(column), escape(column.title)))
            .collect::<String>();
        format!("<table>\n<thead><tr>{head}</tr></thead>\n<tbody>\n{body}</tbody>\n</table>\n")
    };
    format!(
        "<section id=\"{}\">\n<h2>{}</h2>\n<p>{}</p>\n{content}</section>\n",
        escape(R::ID),
        escape(R::TITLE),
        escape(R::INTRO)
    )
}

const fn class(column: &Column) -> &'static str {
    if column.numeric { " class=\"num\"" } else { "" }
}

/// Macht einen Text HTML-sicher: die fünf Zeichen, die Markup oder ein Attribut öffnen könnten,
/// werden zu Entitäten.
pub(crate) fn escape(text: &str) -> String {
    text.chars().fold(
        String::with_capacity(text.len()),
        |mut escaped, character| {
            match character {
                '&' => escaped.push_str("&amp;"),
                '<' => escaped.push_str("&lt;"),
                '>' => escaped.push_str("&gt;"),
                '"' => escaped.push_str("&quot;"),
                '\'' => escaped.push_str("&#39;"),
                other => escaped.push(other),
            }
            escaped
        },
    )
}

const HEAD: &str = "<!DOCTYPE html>
<html lang=\"de\">
<head>
<meta charset=\"utf-8\">
<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">
<title>Bank-Auswertung</title>
<style>
:root { color-scheme: light dark; --rand: #d0d4da; --zebra: #f4f6f8; --warn: #fff4e5; --warntext: #7a4500; }
@media (prefers-color-scheme: dark) { :root { --rand: #3a3f46; --zebra: #1d2126; --warn: #3a2a12; --warntext: #ffcf8a; } }
body { font: 15px/1.5 system-ui, -apple-system, \"Segoe UI\", sans-serif; margin: 0; }
header, nav, section, footer, .warnung { max-width: 72rem; margin: 0 auto; padding: 0 1rem; }
h1 { margin: 1.5rem 0 0.25rem; }
h2 { margin: 2rem 0 0.25rem; border-bottom: 1px solid var(--rand); padding-bottom: 0.25rem; }
.lead { margin-top: 0; opacity: 0.8; }
nav { margin-top: 0.5rem; }
nav a { margin-right: 1rem; white-space: nowrap; }
.warnung { background: var(--warn); color: var(--warntext); border-radius: 6px; padding: 0.75rem 1rem; margin-top: 1rem; }
table { border-collapse: collapse; width: 100%; margin: 0.5rem 0 1rem; display: block; overflow-x: auto; }
th, td { text-align: left; padding: 0.3rem 0.6rem; border-bottom: 1px solid var(--rand); vertical-align: top; }
th { font-weight: 600; }
tbody tr:nth-child(even) { background: var(--zebra); }
.num { text-align: right; white-space: nowrap; font-variant-numeric: tabular-nums; }
.leer { opacity: 0.7; font-style: italic; }
footer { margin: 2rem auto; font-size: 0.85rem; opacity: 0.75; }
</style>
</head>
<body>
";

const NAV: &str = "<nav><a href=\"#cashflow\">Cashflow</a><a href=\"#umbuchungen\">Umbuchungen</a>\
<a href=\"#gebuehren\">Gebühren</a><a href=\"#ranking\">Ranking</a>\
<a href=\"#spar-kandidaten\">Spar-Kandidaten</a><a href=\"#wiederkehr\">Wiederkehr</a></nav>\n";

const FOOT: &str = "<footer>Konten erscheinen nur über ihre Rolle, Privatpersonen nur als \
Pseudonym, Firmen über ihren Namen oder ihre Gläubiger-ID. Eine Kategorie stammt aus dem \
Kategorie-Register; „unbekannt“ heißt: noch keine gelernt. Der Verwendungszweck wird heuristisch \
gescrubbt — nicht beweisbar vollständig.</footer>
</body>
</html>
";

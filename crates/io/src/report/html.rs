//! Das **HTML** des Berichts: eine eigenständige Seite — CSS inline, keine externe Ressource, kein
//! JavaScript. Kein Template-Crate: die Seite ist eine feste Folge von Abschnitten und Tabellen, und
//! **jeder** Text geht durch [`escape`] — auch die Kategorie-Etiketten, die aus einer Chat-Antwort
//! stammen (Issue 0011) und nur auf Länge und Zeichenklasse geprüft sind, nicht auf HTML.

use crate::report::model::Bericht;

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

impl Bericht {
    /// Rendert den Bericht als vollständige HTML-Seite.
    pub(crate) fn render(&self) -> String {
        let sections = [
            section(
                "cashflow",
                "Cashflow je Konto und Monat",
                "Was auf jedem eigenen Konto ein- und abging — Umbuchungen zwischen den eigenen Konten \
                 sind herausgerechnet, Gebühren sind enthalten.",
                &table(
                    &[
                        text("Konto"),
                        text("Monat"),
                        number("Eingang"),
                        number("Ausgang"),
                    ],
                    self.cashflows.iter().map(|line| {
                        vec![
                            line.account.as_str(),
                            line.month.as_str(),
                            line.incoming.as_str(),
                            line.outgoing.as_str(),
                        ]
                    }),
                ),
            ),
            section(
                "umbuchungen",
                "Umbuchungen zwischen eigenen Konten",
                "Was aus dem Cashflow herausgerechnet wurde: ein Ausgang ging auf das genannte \
                 eigene Konto, ein Eingang kam von dort.",
                &table(
                    &[
                        text("Monat"),
                        text("Gegenkonto"),
                        number("Eingang"),
                        number("Ausgang"),
                    ],
                    self.transfers.iter().map(|line| {
                        vec![
                            line.month.as_str(),
                            line.counter_account.as_str(),
                            line.incoming.as_str(),
                            line.outgoing.as_str(),
                        ]
                    }),
                ),
            ),
            section(
                "gebuehren",
                "Gebühren",
                "Was die Bank selbst eingezogen hat — echter Abfluss, darum auch im Cashflow \
                 enthalten.",
                &table(
                    &[
                        text("Monat"),
                        text("Konto"),
                        number("Eingezogen"),
                        number("Erstattet"),
                    ],
                    self.fees.iter().map(|line| {
                        vec![
                            line.month.as_str(),
                            line.account.as_str(),
                            line.charged.as_str(),
                            line.refunded.as_str(),
                        ]
                    }),
                ),
            ),
            section(
                "ranking",
                "Pareto-Ranking der Kostentreiber",
                "Die größten Abflüsse zuerst. Der kumulierte Anteil zeigt, wie wenige Treiber den \
                 Großteil ausmachen.",
                &table(
                    &[
                        number("Platz"),
                        text("Treiber"),
                        text("Kategorie"),
                        number("Abfluss"),
                        number("Buchungen"),
                        number("Anteil"),
                        number("Kumuliert"),
                        number("Jahresbetrag"),
                        text("Takt"),
                    ],
                    self.ranking.iter().map(|line| {
                        vec![
                            line.rank.as_str(),
                            line.driver.as_str(),
                            line.category.as_str(),
                            line.outflow.as_str(),
                            line.bookings.as_str(),
                            line.share.as_str(),
                            line.cumulative_share.as_str(),
                            line.annual.as_str(),
                            line.rhythm.as_str(),
                        ]
                    }),
                ),
            ),
            section(
                "spar-kandidaten",
                "Spar-Kandidaten",
                "Treiber, deren Jahresbetrag die Schwelle aus der Spar-Config erreicht — ein \
                 Vorschlag, entschieden wird von dir.",
                &table(
                    &[
                        number("Platz"),
                        text("Treiber"),
                        text("Kategorie"),
                        number("Jahresbetrag"),
                        number("Anteil"),
                        text("Takt"),
                        text("Preissteigerung"),
                    ],
                    self.savings.iter().map(|line| {
                        vec![
                            line.rank.as_str(),
                            line.driver.as_str(),
                            line.category.as_str(),
                            line.annual.as_str(),
                            line.share.as_str(),
                            line.rhythm.as_str(),
                            line.price_increase.as_str(),
                        ]
                    }),
                ),
            ),
            section(
                "wiederkehr",
                "Wiederkehrende Abflüsse",
                "Abos, Verträge, Daueraufträge: monatlich oder jährlich, mit Jahresbetrag zum \
                 aktuellen Preis. Händler hinter einem Zahlungsdienst stehen einzeln.",
                &table(
                    &[
                        text("Treiber"),
                        text("Kategorie"),
                        text("Takt"),
                        number("Je Abbuchung"),
                        number("Jahresbetrag"),
                        text("Preissteigerung"),
                        text("Konto"),
                    ],
                    self.recurrences.iter().map(|line| {
                        vec![
                            line.driver.as_str(),
                            line.category.as_str(),
                            line.rhythm.as_str(),
                            line.typical.as_str(),
                            line.annual.as_str(),
                            line.price_increase.as_str(),
                            line.account.as_str(),
                        ]
                    }),
                ),
            ),
        ];

        [
            HEAD.to_string(),
            format!(
                "<header><h1>Bank-Auswertung</h1><p class=\"lead\">Wohin das Geld fließt — \
                 errechnet aus den eingelesenen Kontoauszügen. Währung: {}.</p></header>\n",
                escape(&self.currency)
            ),
            self.notice(),
            NAV.to_string(),
            sections.concat(),
            FOOT.to_string(),
        ]
        .concat()
    }

    /// Der Hinweis oben im Bericht, wenn der Analyse Buchungen fehlen (ADR-0017) — sonst nichts.
    fn notice(&self) -> String {
        if self.missing_bookings == 0 {
            return String::new();
        }
        format!(
            "<p class=\"warnung\" role=\"alert\"><strong>Analyse unvollständig:</strong> {} \
             Buchung(en) fehlen. Sie wurden beim Verarbeiten ausgesondert (siehe Lauf-Protokoll) und \
             sind in keiner Zahl dieses Berichts enthalten.</p>\n",
            self.missing_bookings
        )
    }
}

/// Ein Abschnitt mit Überschrift, einem Satz Erklärung und seinem Inhalt (bereits HTML).
fn section(id: &str, title: &str, intro: &str, body: &str) -> String {
    format!(
        "<section id=\"{}\">\n<h2>{}</h2>\n<p>{}</p>\n{body}</section>\n",
        escape(id),
        escape(title),
        escape(intro)
    )
}

/// Eine Tabelle; jede Zelle wird escaped. Ohne Zeilen steht statt der Tabelle ein Satz.
fn table<'a>(columns: &[Column], rows: impl Iterator<Item = Vec<&'a str>>) -> String {
    let body = rows
        .map(|cells| {
            let cells = columns
                .iter()
                .zip(cells)
                .map(|(column, cell)| format!("<td{}>{}</td>", class(column), escape(cell)))
                .collect::<String>();
            format!("<tr>{cells}</tr>\n")
        })
        .collect::<String>();
    if body.is_empty() {
        return "<p class=\"leer\">Keine Einträge.</p>\n".to_string();
    }
    let head = columns
        .iter()
        .map(|column| format!("<th{}>{}</th>", class(column), escape(column.title)))
        .collect::<String>();
    format!("<table>\n<thead><tr>{head}</tr></thead>\n<tbody>\n{body}</tbody>\n</table>\n")
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

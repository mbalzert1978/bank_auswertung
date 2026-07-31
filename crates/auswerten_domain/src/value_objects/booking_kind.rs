use crate::value_objects::NonEmptyString;

/// Die **Marker im Buchungstext**, an denen eine Bank-Gebühr zu erkennen ist — geprüft als
/// Teilzeichenkette des großgeschriebenen Texts, damit `ENTGELT` auch `ENTGELTABSCHLUSS` und
/// `KONTOFUEHRUNGSENTGELT` erwischt.
///
/// Der Buchungstext ist ein **von der Bank vergebenes Vokabular**, kein Freitext: er benennt die
/// *Art* der Buchung (`LASTSCHRIFT`, `DAUERAUFTRAG`, `ENTGELTABSCHLUSS`), nicht die Gegenpartei.
/// Darum ist er hier auswertbar, ohne die Datenschutz-Grenze anzutasten — er trägt keinen
/// Identifikator, den man re-identifizieren könnte.
///
/// **Bewusst kurz und bewusst konservativ.** Ein `ABSCHLUSS` allein steht auch am
/// *Rechnungs*abschluss, der Zinsen bucht — er stünde hier für Geld, das die Bank gar nicht
/// einzieht. Lieber eine Gebühr **nicht** erkennen (sie bleibt dann ein gewöhnlicher Abfluss und
/// fehlt nur in der Gebühren-Sicht) als eine Nicht-Gebühr als solche etikettieren: das erste ist
/// eine Lücke, das zweite eine falsche Aussage darüber, was die Bank kostet.
///
/// **Offene HITL-Lücke:** die Liste stammt aus dem Vokabular der Fixture-Auszüge und aus
/// allgemeiner Kenntnis, **nicht** aus einer Sichtung von Markus' echten Auszügen — anders als die
/// Gläubiger-IDs der Durchleiter, die er abgelesen hat. Welche Buchungstexte seine Bank für
/// Gebühren tatsächlich setzt, ist noch zu verifizieren
/// ([Issue 0006](../../../../docs/issues/0006-gebuehren-klassifikation.md)).
const FEE_MARKERS: [&str; 3] = ["ENTGELT", "GEBUEHR", "GEBÜHR"];

/// Die **Buchungsart** — wofür eine Buchung im Abfluss steht.
///
/// Ein **Summentyp** (ADR-0005) statt eines `ist_gebuehr`-Flags: die Frage „was für eine Buchung ist
/// das" hat mehr als zwei Antworten, sobald Zinsen oder Stornos dazukommen, und ein `bool` müsste
/// dafür an jeder Aufrufstelle aufgebrochen werden.
///
/// **Sie ersetzt den Abfluss nicht, sie deutet ihn.** Eine Gebühr ist echtes, verlorenes Geld und
/// bleibt darum im Cashflow — anders als die Umbuchung, die kein Abfluss ist und deshalb aus ihm
/// herausfällt ([Issue 0004](../../../../docs/issues/0004-umbuchung-netting.md)). Die Gebühren-Sicht
/// ist eine **Überlagerung** des Cashflows, keine zweite Hälfte davon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BookingKind {
    /// Was die **Bank selbst** einzieht — Kontoführung, Auslandseinsatz, die Gebühr *für* eine
    /// Umbuchung (CONTEXT.md, „Umbuchungskosten": der Hauptverdächtige des Projekts).
    Fee,
    /// Jede andere Buchung — das, was der Nutzer selbst veranlasst hat.
    Regular,
}

impl BookingKind {
    /// Die **reine Funktion** `Buchungstext → Buchungsart`. Kein Zustand, kein Port, kein I/O —
    /// dieselbe Eingabe ergibt immer dieselbe Ausgabe (dieselbe Linie wie beim
    /// [`Fingerprint`](crate::Fingerprint): Fachwissen der Domäne, keine Konfiguration).
    ///
    /// **Ohne Buchungstext keine Gebühr.** Eine Zeile, die keinen führt, wird als
    /// [`Regular`](Self::Regular) gedeutet: nichts an ihr sagt, dass die Bank hier etwas eingezogen
    /// hätte, und sie im Zweifel zur Gebühr zu erklären hieße, die Bank für fremde Ausgaben
    /// verantwortlich zu machen.
    ///
    /// **Heuristisch, nicht beweisbar vollständig** — sie erkennt die Gebühren, deren Marker in
    /// [`FEE_MARKERS`] stehen, und behauptet nicht, jede zu finden.
    #[must_use]
    pub(crate) fn of(booking_text: Option<&NonEmptyString>) -> Self {
        match booking_text {
            Some(text) => {
                let normalised = text.as_str().to_uppercase();
                match FEE_MARKERS.iter().any(|marker| normalised.contains(marker)) {
                    true => Self::Fee,
                    false => Self::Regular,
                }
            }
            None => Self::Regular,
        }
    }
}

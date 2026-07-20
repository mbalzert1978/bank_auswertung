use std::collections::BTreeMap;

use crate::value_objects::{
    CategoryRequest, Currency, Fingerprint, NonEmptyString, SeedMarker, WindowedBooking,
};

/// Das **eingefrorene Auswertungsfenster**, wie dieser Slice es **per ID** zurückliest — die
/// Einheit, über die die periodische Auswertung **innerhalb** rechnet (ADR-0013, Entscheidung E,
/// Reichweite 1). Es trägt die **eine** Währung des Fensters und seine Buchungen.
///
/// **Rehydratisierung, keine Neukonstruktion:** die Invarianten des Fensters (nicht leer, eine
/// Währung) sind schon **an der Konstruktion** in `fenstern` entstanden (ADR-0013, Entscheidung A).
/// Dieser Slice liest einen Beweis, der bereits geführt ist — ihn hier erneut zu führen wäre eine
/// zweite, divergierbare Kopie derselben Regel. Was er stattdessen **nicht** kann, ist ein Fenster
/// erfinden: das Aggregat entsteht nur aus dem, was die Ablage hergibt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalysisWindow {
    currency: Currency,
    /// Das Kennzeichen der Tokenisierung, unter der die Token dieses Fensters entstanden sind —
    /// `obfuskieren` hat sie vergeben, io reicht das Etikett mit.
    seed_marker: SeedMarker,
    bookings: Vec<WindowedBooking>,
}

impl AnalysisWindow {
    /// **Hydratisiert** das Fenster aus dem, was der Lese-Port zur ID geliefert hat.
    #[must_use]
    pub fn rehydrate(
        currency: Currency,
        seed_marker: SeedMarker,
        bookings: Vec<WindowedBooking>,
    ) -> Self {
        Self {
            currency,
            seed_marker,
            bookings,
        }
    }

    pub(crate) fn seed_marker(&self) -> &SeedMarker {
        &self.seed_marker
    }

    pub(crate) fn currency(&self) -> &Currency {
        &self.currency
    }

    pub(crate) fn bookings(&self) -> &[WindowedBooking] {
        &self.bookings
    }

    /// Die **Anfragen**, die dieses Fenster ans Orakel stellen *könnte* — je **distinkter
    /// Buchungsart genau eine** (welche davon wirklich hinausgehen, entscheidet erst der Abgleich
    /// mit dem Kategorie-Register).
    ///
    /// **Die Deduplizierung ist der Punkt, nicht ein Nebeneffekt.** Zwanzig Buchungen desselben
    /// Händlers sind **eine** Art und damit **eine** Frage; das `BTreeMap` über dem Fingerabdruck
    /// faltet sie zusammen und legt die Anfragen zugleich in stabiler, eingabe-unabhängiger
    /// Reihenfolge ab. Ohne das wüchse der Chat-Payload mit der Zahl der Buchungen statt mit der
    /// Zahl der Arten — und das Orakel bekäme dieselbe Frage zwanzigmal.
    ///
    /// Zwei Fälle tragen **nichts** bei: eine Buchung ohne Gegenpartei (Entgeltzeile) hat keinen
    /// Fingerabdruck, und eine ohne Text hat nichts, woran sich eine Kategorie erraten ließe — eine
    /// leere Frage zu stellen wäre Egress ohne Erkenntnis.
    pub(crate) fn category_requests(&self) -> Vec<CategoryRequest> {
        self.bookings
            .iter()
            // `?` statt `zip`: der Schlüssel entscheidet **zuerst**, und für eine Buchung ohne ihn
            // (Entgeltzeile) wird der Text gar nicht erst gebaut. `Option::zip` wertet sein Argument
            // eifrig aus und hätte ihn auch dort zusammengesetzt, wo er niemanden interessiert.
            .filter_map(|booking| {
                let fingerprint = Fingerprint::of(booking.recipient(), booking.purpose())?;
                Some((fingerprint, category_text(booking)?))
            })
            // `or_insert` statt Überschreiben: der **erste** Text einer Art gewinnt. Welcher es ist,
            // darf nicht davon abhängen, in welcher Reihenfolge die Ablage die Zeilen liefert.
            .fold(BTreeMap::new(), |mut requests, (fingerprint, text)| {
                requests.entry(fingerprint).or_insert(text);
                requests
            })
            .into_iter()
            .map(|(fingerprint, text)| CategoryRequest::new(fingerprint, text))
            .collect()
    }
}

/// Der **obfuskierte Text** einer Buchung, an dem das Orakel die Kategorie erraten soll: Empfänger
/// **und** Verwendungszweck, so wie CONTEXT.md („Kategorie") sie nennt — der Firmenname als
/// wichtigstes, der Freitext als zweitwichtigstes Signal.
///
/// Beide zusammen in **einem** Feld, nicht als zwei: das Orakel bekommt einen Text zum Deuten, und
/// welche Bestandteile ihn ergeben, ist eine Aussage der Domäne über das Kategorie-Signal — keine
/// Struktur, die das DTO nach außen tragen müsste.
///
/// Liegt keines von beiden vor, gibt es **nichts zu fragen** (`None`).
fn category_text(booking: &WindowedBooking) -> Option<NonEmptyString> {
    let signals = [booking.recipient().category_signal(), booking.purpose()]
        .into_iter()
        .flatten()
        .map(NonEmptyString::as_str)
        .collect::<Vec<_>>();
    NonEmptyString::new(&signals.join(" "))
}

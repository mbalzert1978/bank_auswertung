use std::cell::RefCell;

use crate::abstractions::StoreSeam;
use crate::dto::Line;

/// Beobachtet die **gestageten** Zeilen an der Speicher-Grenze: hält je `store` das **ganze**
/// [`Line`]-DTO **samt der vergebenen ID** fest, sodass ein Spec sowohl die projizierten Felder
/// (Feld-Treue inkl. Währung und Optional-`none`) als auch belegen kann, dass die Roh-Zeile unter
/// **genau der** ID liegt, die als Zeiger publiziert wird. Implementiert die öffentliche Abstraction
/// [`StoreSeam`] (nicht den Domänen-Port direkt) — hinter dem echten Seam steckt später sqlx in io,
/// das die ID `INSERT … RETURNING id` vergibt. Beobachtet wird das **Staging**; ein Commit findet
/// hier bewusst nicht statt (io-Sache).
///
/// **Die vergebene ID** ist deterministisch (`ingested-1`, `ingested-2`, … in Staging-Reihenfolge) —
/// so kann ein Spec die publizierten Zeiger und die Speicher-Schlüssel exakt gegeneinander prüfen.
#[derive(Default)]
pub(crate) struct FakeStoreSeam {
    rows: RefCell<Vec<(String, Line)>>,
}

impl FakeStoreSeam {
    /// Die gestageten Zeilen in Lesereihenfolge — das konkrete DTO am Seam (kein generisches
    /// Serialisieren), an dem ein Spec die Feld-Treue der Projektion assertiert.
    pub(crate) fn lines(&self) -> Vec<Line> {
        self.rows
            .borrow()
            .iter()
            .map(|(_, line)| line.clone())
            .collect()
    }

    /// Die `ordering_account`-Werte der gestageten Zeilen — bequemer Kurzzugriff für Specs, die
    /// nur die Persistenz-Zählung je Zeile prüfen.
    pub(crate) fn ordering_accounts(&self) -> Vec<String> {
        self.rows
            .borrow()
            .iter()
            .map(|(_, line)| line.ordering_account.clone())
            .collect()
    }

    /// Die **vergebenen IDs**, unter denen die Zeilen abgelegt wurden, in Staging-Reihenfolge — ein
    /// Spec prüft damit, dass die publizierten Zeiger genau auf die abgelegten Roh-Zeilen zeigen.
    pub(crate) fn stored_ids(&self) -> Vec<String> {
        self.rows
            .borrow()
            .iter()
            .map(|(id, _)| id.clone())
            .collect()
    }
}

impl StoreSeam for FakeStoreSeam {
    fn store(&self, line: &Line) -> String {
        let mut rows = self.rows.borrow_mut();
        // Deterministische, laufübergreifend eindeutige ID (der Fake spielt `INSERT … RETURNING id`).
        let id = format!("ingested-{}", rows.len() + 1);
        rows.push((id.clone(), line.clone()));
        id
    }
}

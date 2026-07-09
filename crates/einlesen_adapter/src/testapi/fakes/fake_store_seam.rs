use std::cell::RefCell;

use crate::abstractions::StoreSeam;
use crate::dto::Line;

/// Beobachtet die **gestageten** Zeilen an der Speicher-Grenze: hält je `store` das **ganze**
/// [`Line`]-DTO fest, sodass ein Spec die projizierten Felder (Feld-Treue inkl. Währung und
/// Optional-`none`) am realen Durchlauf prüfen kann. Implementiert die öffentliche Abstraction
/// [`StoreSeam`] (nicht den Domänen-Port direkt) — hinter dem echten Seam steckt später sqlx in
/// io. Beobachtet wird das **Staging**; ein Commit findet hier bewusst nicht statt (io-Sache).
#[derive(Default)]
pub(crate) struct FakeStoreSeam {
    lines: RefCell<Vec<Line>>,
}

impl FakeStoreSeam {
    /// Die gestageten Zeilen in Lesereihenfolge — das konkrete DTO am Seam (kein generisches
    /// Serialisieren), an dem ein Spec die Feld-Treue der Projektion assertiert.
    pub(crate) fn lines(&self) -> Vec<Line> {
        self.lines.borrow().clone()
    }

    /// Die `ordering_account`-Werte der gestageten Zeilen — bequemer Kurzzugriff für Specs, die
    /// nur die Persistenz-Zählung je Zeile prüfen.
    pub(crate) fn ordering_accounts(&self) -> Vec<String> {
        self.lines
            .borrow()
            .iter()
            .map(|line| line.ordering_account.clone())
            .collect()
    }
}

impl StoreSeam for FakeStoreSeam {
    fn store(&self, line: &Line) {
        self.lines.borrow_mut().push(line.clone());
    }
}

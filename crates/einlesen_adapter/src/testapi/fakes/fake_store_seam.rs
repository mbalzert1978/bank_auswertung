use std::cell::RefCell;

use crate::abstractions::StoreSeam;
use crate::dto::Line;

/// Beobachtet die **gestageten** Zeilen an der Speicher-Grenze: hält je `store` das
/// `ordering_account` fest, sodass ein Spec die persistierten Aggregate zählen/prüfen kann.
/// Implementiert die öffentliche Abstraction [`StoreSeam`] (nicht den Domänen-Port direkt) —
/// hinter dem echten Seam steckt später sqlx in io. Beobachtet wird das **Staging**; ein Commit
/// findet hier bewusst nicht statt (io-Sache).
#[derive(Default)]
pub(crate) struct FakeStoreSeam {
    ordering_accounts: RefCell<Vec<String>>,
}

impl FakeStoreSeam {
    pub(crate) fn ordering_accounts(&self) -> Vec<String> {
        self.ordering_accounts.borrow().clone()
    }
}

impl StoreSeam for FakeStoreSeam {
    fn store(&self, line: &Line) {
        self.ordering_accounts
            .borrow_mut()
            .push(line.ordering_account.clone());
    }
}

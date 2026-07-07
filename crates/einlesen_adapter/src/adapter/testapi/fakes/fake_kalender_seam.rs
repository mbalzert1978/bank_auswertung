use crate::abstractions::{KalenderSeam, Kalendertag};

/// Deterministischer Fake: deutet „TT.MM.JJJJ" zu seinen Bestandteilen; ungültiges Format ergibt
/// `None`. Implementiert den öffentlichen [`KalenderSeam`] (nicht den Domänen-Port) — hinter dem
/// echten Seam steckt später `chrono` in IO (Vorbild `FakeSftpPuller : ISftpPuller`).
pub(crate) struct FakeKalenderSeam;

impl KalenderSeam for FakeKalenderSeam {
    fn deute(&self, roh: &str) -> Option<Kalendertag> {
        let mut teile = roh.split('.');
        let tag: u8 = teile.next()?.parse().ok()?;
        let monat: u8 = teile.next()?.parse().ok()?;
        let jahr: u16 = teile.next()?.parse().ok()?;
        Some(Kalendertag { jahr, monat, tag })
    }
}

use application_kernel::{Command, CommandDecoder};
use fenstern_domain::{DayStamp, NonEmptyString, RunId};

/// Der Dispatch-Key der **konsumierten** Nachricht — auf ihn löst `io`s Prozessor diesen Slice als
/// Consumer auf. Prosa-Name im ADR: der **Fenster-Schritt**.
///
/// **Besonderheit dieses Slice:** er ist zugleich der Typ, den der Schritt **selbst publiziert**
/// ([`AdvanceWindowMessage`](crate::AdvanceWindowMessage)) — der Pager reiht sein eigenes nächstes
/// Kommando ein, bis die Historie versiegt (ADR-0013, Entscheidung B). Konsumiert und publiziert
/// sind hier **derselbe** Nachrichtentyp; genau das *ist* die Selbst-Fortsetzung.
///
/// Der Name bleibt trotzdem `CONSUMED_MESSAGE_TYPE`: er ist **slice-übergreifende Konvention**
/// (`einlesen`/`obfuskierung` tragen ihn ebenso, und `io` re-exportiert ihn je Slice unter genau
/// diesem Namen, um den Dispatcher darauf zu registrieren). Ihn hier allein umzubenennen bräche die
/// uniforme Auflösung, für die er existiert — die Doppelrolle ist eine Eigenschaft des Pagers, kein
/// Namensfehler.
pub const CONSUMED_MESSAGE_TYPE: &str = "AdvanceWindow";

/// Das interne Kommando am Slice-Eingang: es trägt **immer** einen Anker (ADR-0013, Entscheidung B)
/// — den Tag-Stamp, der die **obere** Grenze der Seite bildet. Der
/// [`WindowingHandler`](crate::WindowingHandler) baut daraus das domänen-interne `AdvanceWindow`.
/// Es erfüllt `application_kernel::Command` (Marker).
pub struct AdvanceWindowCommand {
    anchor: i64,
    run_top: i64,
    run: RunId,
}

impl AdvanceWindowCommand {
    /// Der Anker als Tag-Stamp, den der Handler in einen `DayStamp` hüllt.
    pub(crate) fn anchor(&self) -> DayStamp {
        DayStamp::new(self.anchor)
    }

    /// Der **Lauf-Gipfel** als Tag-Stamp — der Anker, mit dem der Lauf begann. Er reist unverändert
    /// durch die Selbst-Fortsetzung und bildet beim Versiegen der Historie die Obergrenze der
    /// publizierten Lauf-Spanne.
    pub(crate) fn run_top(&self) -> DayStamp {
        DayStamp::new(self.run_top)
    }

    /// Die **Lauf-Identität**, die unverändert durch die Selbst-Fortsetzung reist (ADR-0016).
    pub(crate) fn run(&self) -> &RunId {
        &self.run
    }
}

impl Command for AdvanceWindowCommand {}

/// Der slice-gelieferte [`CommandDecoder`] (reine ACL, Shape Y): `io` extrahiert die Envelope und
/// reicht nur die nackten Strings herein — der Slice nennt **keinen** Outbox-Typ. Der Anker reist
/// als **stringifizierter Tag-Stamp im `partition_key`** (ADR-0013, Entscheidung B: kein serde);
/// der `content` ist leer.
///
/// Der Schlüssel trägt **zwei** Tag-Stamps und die Lauf-Identität in der Form
/// `"{anchor}:{run_top}:{run}"`: den Anker der Seite, den unveränderten **Lauf-Gipfel**, ohne den der
/// terminierende Schritt die Lauf-Spanne nicht kennte (der Handler ist zustandslos), und den **Lauf**,
/// zu dem die Seite gehört (ADR-0016; zuletzt, weil er opak ist). Ein `partition_key`, der diese Form
/// **nicht** trägt (leer, blank, ein Teil fehlt oder ist keine Zahl), ist kein gültiger Anker → `None`, das `io` auf der
/// Envelope-Ebene behandelt (nicht als Handler-Fehler durchgereicht). **Parse, don't validate:** was
/// hier durchkommt, sind echte `i64` — der Handler bekommt nie einen Text, den er noch deuten
/// müsste.
pub struct AdvanceWindowCommandDecoder;

impl CommandDecoder for AdvanceWindowCommandDecoder {
    type Command = AdvanceWindowCommand;

    fn decode(partition_key: &str, _content: &str) -> Option<Self::Command> {
        let mut parts = partition_key.trim().splitn(3, ':');
        let anchor = parts.next()?.trim().parse().ok()?;
        let run_top = parts.next()?.trim().parse().ok()?;
        let run = RunId::from(NonEmptyString::new(parts.next()?)?);
        Some(AdvanceWindowCommand {
            anchor,
            run_top,
            run,
        })
    }
}

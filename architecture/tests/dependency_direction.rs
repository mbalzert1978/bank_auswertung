//! Dependency-Richtungs-Test (ADR-0007): prüft über `cargo_metadata`, dass die
//! Kanten des Workspace-Graphen nach innen zeigen. Jede Regel ist ein eigener
//! `#[test]` — ein deutscher Spezifikationssatz, den ein Architektur-Leser als
//! lebende Doku liest. `architecture` ist der Validator und von allen Regeln
//! ausgenommen (es allein bindet mit `cargo_metadata` eine Fremd-Crate).

use std::collections::BTreeSet;

use cargo_metadata::MetadataCommand;

/// Fremd-Crates, die die "nach innen"-Regel nicht brechen (auch außerhalb des
/// `io`-Crate erlaubt) — deckungsgleich mit dem PreToolUse-Hook.
const SANCTIONED: &[&str] = &["serde", "serde_json", "thiserror", "static_assertions"];

/// Ein Workspace-Crate mit seinen (nach intern/extern geschiedenen) Deps.
struct Crate {
    name: String,
    deps: Vec<Dep>,
}

struct Dep {
    name: String,
    /// `true`, wenn die Dep ein anderes Workspace-Member ist (also intern).
    internal: bool,
}

impl Crate {
    fn internal_deps(&self) -> impl Iterator<Item = &str> {
        self.deps
            .iter()
            .filter(|dep| dep.internal)
            .map(|dep| dep.name.as_str())
    }

    fn external_deps(&self) -> impl Iterator<Item = &str> {
        self.deps
            .iter()
            .filter(|dep| !dep.internal)
            .map(|dep| dep.name.as_str())
    }

    fn dep_names(&self) -> String {
        self.deps
            .iter()
            .map(|dep| dep.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// Liest den aufgelösten Abhängigkeitsgraphen des Workspace über `cargo metadata`.
/// Läuft aus dem Manifest-Verzeichnis dieser Crate — `cargo metadata` findet von
/// dort die Workspace-Wurzel selbst.
fn workspace_crates() -> Vec<Crate> {
    let metadata = MetadataCommand::new()
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .exec()
        .expect("cargo metadata liefert den Workspace-Graphen");

    let members: BTreeSet<String> = metadata
        .workspace_members
        .iter()
        .filter_map(|id| metadata.packages.iter().find(|package| package.id == *id))
        .map(|package| package.name.to_string())
        .collect();

    metadata
        .workspace_members
        .iter()
        .filter_map(|id| metadata.packages.iter().find(|package| package.id == *id))
        .map(|package| Crate {
            name: package.name.to_string(),
            deps: package
                .dependencies
                .iter()
                .map(|dep| Dep {
                    name: dep.name.to_string(),
                    internal: members.contains(dep.name.as_str()),
                })
                .collect(),
        })
        .collect()
}

/// `Some(slice)`, wenn `crate_name` auf `layer_suffix` endet (z. B.
/// `"csv_camt_v8_adapter"` + `"_adapter"` → `"csv_camt_v8"`).
fn slice_of<'a>(crate_name: &'a str, layer_suffix: &str) -> Option<&'a str> {
    crate_name.strip_suffix(layer_suffix)
}

fn assert_no_violations(violations: &[String], rule: &str) {
    assert!(
        violations.is_empty(),
        "Dependency-Richtung verletzt — {rule}:\n{}",
        violations.join("\n")
    );
}

#[test]
fn domaenen_crates_sind_dependency_frei() {
    let crates = workspace_crates();
    let violations: Vec<String> = crates
        .iter()
        .filter(|item| item.name.ends_with("_domain"))
        .filter(|item| !item.deps.is_empty())
        .map(|item| {
            format!(
                "`{}` bindet {} — muss dependency-frei (blind) sein",
                item.name,
                item.dep_names()
            )
        })
        .collect();
    assert_no_violations(&violations, "_domain ist blind (keine Deps)");
}

const SHARED_KERNEL: &str = "shared_kernel";
const APPLICATION_KERNEL: &str = "application_kernel";

fn assert_interface_only(crate_name: &str) {
    let crates = workspace_crates();
    let violations: Vec<String> = crates
        .iter()
        .filter(|item| item.name == crate_name)
        .filter(|item| !item.deps.is_empty())
        .map(|item| {
            format!(
                "`{crate_name}` bindet {} — muss interface-only bleiben",
                item.dep_names()
            )
        })
        .collect();
    assert_no_violations(
        &violations,
        &format!("{crate_name} ist interface-only (keine Deps)"),
    );
}

#[test]
fn shared_kernel_ist_interface_only() {
    assert_interface_only(SHARED_KERNEL);
}

#[test]
fn application_kernel_ist_interface_only() {
    assert_interface_only(APPLICATION_KERNEL);
}

#[test]
fn adapter_binden_kein_shared_kernel() {
    // Nach der Migration des zweiten Slice (Issue 0021) ist **kein** `<slice>_adapter` mehr
    // outbox-koppelnd: die Outbox-Mechanik (`shared_kernel`: `OutboxMessage`/`OutboxError`/
    // `OutboxHandler`/`OutboxPublisher`/…) lebt vollständig in `io`. Diese Regel verbietet objektiv
    // **nur** die Kante `_adapter → shared_kernel` (ADR-0011, Entscheidung D); die übrigen Deps eines
    // Adapters (sein `_domain`, der Slice↔io-Vertrag `application_kernel`, sanktionierte Crates) regeln
    // andere Tests. In Issue 0020 bewusst zurückgestellt, solange `obfuskierung` noch auf seiner
    // eigenen Consume-Bridge lief — greift jetzt.
    let crates = workspace_crates();
    let violations: Vec<String> = crates
        .iter()
        .filter(|item| item.name.ends_with("_adapter"))
        .flat_map(|item| {
            item.internal_deps()
                .filter(|dep| *dep == SHARED_KERNEL)
                .map(move |_| {
                    format!(
                        "`{}` bindet `{SHARED_KERNEL}` — ein `_adapter` ist outbox-frei; die Outbox-Mechanik lebt in `io`",
                        item.name,
                    )
                })
        })
        .collect();
    assert_no_violations(
        &violations,
        &format!("kein `_adapter` bindet `{SHARED_KERNEL}`"),
    );
}

#[test]
fn application_kernel_binden_nur_adapter_und_io() {
    // Nach dem Kernel-Flip (ADR-0011, Entscheidung D) ist `application_kernel` der
    // `io`-sichtbare Slice↔`io`-Vertrag: ihn binden **`_adapter` und `io`**, nie `_domain`/
    // `_specs`. (Gelockert gegenüber ADR-0009 „nur `_adapter`", weil `io`s generische
    // `OutboxConsumer<H: Handler>` den `application_kernel::Handler` sehen muss.)
    let crates = workspace_crates();
    let violations: Vec<String> = crates
        .iter()
        .filter(|item| item.name != "architecture")
        .flat_map(|item| {
            item.internal_deps()
                .filter(|dep| *dep == APPLICATION_KERNEL)
                .filter(move |_| !item.name.ends_with("_adapter") && item.name != "io")
                .map(move |_| {
                    format!(
                        "`{}` hängt an `{APPLICATION_KERNEL}` — den Slice↔io-Vertrag binden nur ein `_adapter` und `io`, nie `_domain`/`_specs`",
                        item.name,
                    )
                })
        })
        .collect();
    assert_no_violations(
        &violations,
        &format!("`{APPLICATION_KERNEL}` binden nur `_adapter` und `io`"),
    );
}

#[test]
fn nur_das_io_crate_bindet_externe_fremd_crates() {
    let crates = workspace_crates();
    let violations: Vec<String> = crates
        .iter()
        .filter(|item| item.name != "architecture" && item.name != "io")
        .flat_map(|item| {
            item.external_deps()
                .filter(|dep| !SANCTIONED.contains(dep))
                .map(move |dep| format!("`{}` bindet externe Fremd-Crate `{dep}`", item.name))
        })
        .collect();
    assert_no_violations(
        &violations,
        "nur `io` bindet externe Crates (außer sanktioniert)",
    );
}

#[test]
fn kein_crate_haengt_am_domain_eines_fremden_slice() {
    let crates = workspace_crates();
    let violations: Vec<String> = crates
        .iter()
        .filter(|item| item.name != "architecture")
        .flat_map(|item| {
            item.internal_deps().filter_map(move |dep| {
                let slice = slice_of(dep, "_domain")?;
                let erlaubt = item.name == format!("{slice}_adapter");
                (!erlaubt).then(|| {
                    format!(
                        "`{}` hängt direkt an `{dep}` — eine Fremd-Domäne erreicht man nur über deren `_adapter`-Port",
                        item.name
                    )
                })
            })
        })
        .collect();
    assert_no_violations(
        &violations,
        "kein Zugriff auf eine Fremd-Domäne (Domäne nur über eigenen `_adapter`)",
    );
}

#[test]
fn specs_haengen_nur_am_eigenen_adapter() {
    // Seit beide Slices auf **Handler-Ebene** über den `application_kernel`-Vertrag spezifizieren und
    // gegen den **nackten Domänen-Fehler** (vom Adapter re-exportiert) assertieren (ADR-0011, Issues
    // 0020/0021), bindet **keine** Spec mehr `shared_kernel`: die frühere io-facing-Lockerung ist
    // entfallen — eine Blackbox-Spec hängt genau am eigenen `_adapter`, sonst nichts.
    let crates = workspace_crates();
    let violations: Vec<String> = crates
        .iter()
        .filter_map(|item| slice_of(&item.name, "_specs").map(|slice| (item, slice)))
        .flat_map(|(item, slice)| {
            let erwartet = format!("{slice}_adapter");
            item.deps
                .iter()
                .filter(move |dep| dep.name != erwartet)
                .map(move |dep| {
                    format!(
                        "`{}` hängt an `{}`, erlaubt ist nur `{}_adapter`",
                        item.name, dep.name, slice
                    )
                })
        })
        .collect();
    assert_no_violations(&violations, "_specs hängt nur am eigenen `_adapter`");
}

/// Die internen Crates, die `io` **neben** den Slice-Adaptern binden darf: die beiden Kernel.
/// `application_kernel` ist der Slice↔`io`-Vertrag (`OutboxConsumer<H: Handler>`), `shared_kernel`
/// die io-private Outbox-Mechanik — beide sind Geschwister, `io` bindet sie (ADR-0011).
const IO_ZUSATZ_DEPS: &[&str] = &[APPLICATION_KERNEL, SHARED_KERNEL];

#[test]
fn io_haengt_nur_an_adaptern_und_kerneln_nie_an_domaenen() {
    let crates = workspace_crates();
    let violations: Vec<String> = crates
        .iter()
        .filter(|item| item.name == "io" || item.name.ends_with("_io"))
        .flat_map(|item| {
            item.internal_deps().filter_map(move |dep| {
                if dep.ends_with("_adapter") || IO_ZUSATZ_DEPS.contains(&dep) {
                    None
                } else {
                    Some(format!(
                        "`{}` hängt an `{dep}` — `io` darf nur an `_adapter`-Crates und die Kernel hängen, nie an einer Domäne",
                        item.name
                    ))
                }
            })
        })
        .collect();
    assert_no_violations(
        &violations,
        "`io` hängt nur an `_adaptern` und den Kerneln (nie an einer Domäne)",
    );
}

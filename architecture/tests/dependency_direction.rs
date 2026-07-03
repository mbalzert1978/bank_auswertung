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

#[test]
fn shared_kernel_ist_interface_only() {
    let crates = workspace_crates();
    let violations: Vec<String> = crates
        .iter()
        .filter(|item| item.name == "shared_kernel")
        .filter(|item| !item.deps.is_empty())
        .map(|item| {
            format!(
                "`shared_kernel` bindet {} — muss interface-only bleiben",
                item.dep_names()
            )
        })
        .collect();
    assert_no_violations(&violations, "shared_kernel ist interface-only (keine Deps)");
}

#[test]
fn application_kernel_ist_slice_intern_nur_adapter_haengt_daran() {
    let crates = workspace_crates();
    let violations: Vec<String> = crates
        .iter()
        .filter(|item| item.name != "architecture")
        .flat_map(|item| {
            item.internal_deps()
                .filter(|dep| *dep == "application_kernel")
                .filter(move |_| !item.name.ends_with("_adapter"))
                .map(move |_| {
                    format!(
                        "`{}` hängt an `application_kernel` — die slice-internen Handler-Konventionen sieht nur ein `_adapter`, nie `io`/`_domain`/`_specs`",
                        item.name
                    )
                })
        })
        .collect();
    assert_no_violations(
        &violations,
        "`application_kernel` ist slice-intern (nur `_adapter` bindet ihn)",
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
    let crates = workspace_crates();
    let violations: Vec<String> = crates
        .iter()
        .filter_map(|item| slice_of(&item.name, "_specs").map(|slice| (item, slice)))
        .flat_map(|(item, slice)| {
            let erwartet = format!("{slice}_adapter");
            item.deps.iter().filter_map(move |dep| {
                if dep.name == erwartet {
                    None
                } else {
                    Some(format!(
                        "`{}` hängt an `{}`, erlaubt ist nur `{erwartet}`",
                        item.name, dep.name
                    ))
                }
            })
        })
        .collect();
    assert_no_violations(&violations, "_specs hängt nur am eigenen `_adapter`");
}

#[test]
fn io_haengt_nur_an_adaptern_nie_an_domaenen() {
    let crates = workspace_crates();
    let violations: Vec<String> = crates
        .iter()
        .filter(|item| item.name == "io" || item.name.ends_with("_io"))
        .flat_map(|item| {
            item.internal_deps().filter_map(move |dep| {
                if dep.ends_with("_adapter") {
                    None
                } else {
                    Some(format!(
                        "`{}` hängt an `{dep}` — `io` darf nur an `_adapter`-Crates hängen",
                        item.name
                    ))
                }
            })
        })
        .collect();
    assert_no_violations(
        &violations,
        "`io` hängt nur an `_adaptern` (nie an einer Domäne)",
    );
}

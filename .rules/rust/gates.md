# Die sechs Gates

## Alle sechs Gates grün vor Commit/PR

`build`, `test`, `clippy --all-targets`, `fmt --check`, `cargo test -p architecture` und
`cargo doc --no-deps --workspace` müssen grün sein. Jedes Member erbt das strikte
`[workspace.lints.clippy]`-„Don't Panic"-Set via `[lints] workspace = true`. Prüfe sie über
den Skill **`/cargo-six-gates`** — er führt alle sechs aus und meldet PASS/FAIL je Gate mit
angehängter Fehlerausgabe; die Befehlskette unten ist die von Hand ausgeführte Entsprechung.

Do:

```text
/cargo-six-gates    # kanonischer Weg: alle sechs Gates, PASS/FAIL je Gate
```

Äquivalent von Hand (vom Repo-Root):

```bash
cargo build && cargo test && cargo clippy --all-targets \
  && cargo fmt --check && cargo test -p architecture \
  && cargo doc --no-deps --workspace
```

Das Repo-`.cargo/config.toml` setzt `rustflags = ["-Dwarnings"]` (Scope
`cfg(all())`), sodass **jeder** Workspace-`cargo`-Lauf Warnungen als Fehler behandelt —
`cargo clippy --all-targets` erbt es, ein einzelnes Clippy-Warning macht das Gate also
**rot** (kein Exit 0 trotz Warnung mehr). Ein `-- -D warnings` von Hand ist damit redundant.
Fremd-Crates bleiben durch Cargos automatisches cap-lints unberührt.

Don't:

```bash
cargo build && git commit -m "…"   # Rest der Gates übersprungen
```

## `cargo doc` ist ein echtes Gate — keine toten Doc-Links

`[workspace.lints.rustdoc]` setzt `private_intra_doc_links`/`broken_intra_doc_links` auf
`deny`: öffentliche Doku, die auf ein privates oder nicht existierendes Item verlinkt,
**bricht den Build**. Ein Kollaborateur darf **benannt** werden (Prosa/Code-Auszeichnung),
nur nicht als toter Link.

Do:

```rust
/// Erfüllt den [`RawLineSource`]-Port.        // existierendes, öffentliches Item
/// Kategorisiert über den `Kategorisierer`.   // Prosa/Code-Span, kein Link
```

Don't:

```rust
/// Nutzt den [`ConsumeBridge`].  // privat/nicht existierend → deny, Build bricht
```

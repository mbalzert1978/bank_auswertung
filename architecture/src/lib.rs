//! Top-Validator des Workspace-Abhängigkeitsgraphen (ADR-0007).
//!
//! Diese Crate trägt keinen Laufzeit-Code: sie existiert nur, um über
//! `cargo_metadata` zu prüfen, dass alle Kanten des Workspace-Graphen **nach
//! innen** zeigen (Domäne blind, `io` nur an Adaptern, keine Slice-Slice-Domänen-
//! Kopplung, …). Der eigentliche Test liegt in
//! [`tests/dependency_direction.rs`](../../tests/dependency_direction.rs).
//!
//! `architecture` ist selbst von der Regel ausgenommen — es ist der Validator,
//! kein Slice, und darf als einziges eine nicht-sanktionierte Fremd-Crate binden.

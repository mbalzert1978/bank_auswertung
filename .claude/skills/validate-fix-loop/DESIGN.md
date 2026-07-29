# validate-fix-loop — Entwurf und Begründung

Stand: 2026-07-29. Diese Datei hält fest, **warum** der Loop so geschnitten ist, wie
`SKILL.md` ihn beschreibt. `SKILL.md` sagt, was zu tun ist; hier steht, welche Analyse zu
diesen Entscheidungen geführt hat — damit niemand sie später aus Unkenntnis zurückdreht.

## Der Loop in einem Absatz

23 `verifier-<smell>`-Checks laufen pro Iteration parallel, jeder als eigener Subagent auf
dem in `config.json` gesetzten `agent_model`. Danach routet `fixer_map` die Findings jedes
Verifiers an genau seinen `fixer-<smell>` — ebenfalls als Subagent, gebündelt in Wellen mit
paarweise disjunkten Dateimengen. Der Orchestrator sammelt die Eingaben jeder Welle
unmittelbar vor ihrem Dispatch und fährt das Test-Gate einmal pro Iteration selbst. Ein
Fixer sucht nicht, testet nicht und verifiziert nicht nach.

## Warum nur die 23 Verifier — und nicht die frühere Doppel-Pipeline

Der Loop enthielt zusätzlich acht allgemeine Validatoren (`review-against-rules`,
`lint-and-format-check`, `qa-check`, `architecture-adr-check`, `solid-principles-check`,
`design-pattern-fit-check`, `language-idiom-check`, `illegal-state-check`) plus
`apply-validator-findings` als Sammel-Fixer. Eine Durchsicht **aller** 55 beteiligten
Rubriken ergab drei Befunde.

### 1. `review-against-rules` war ein Superset von fast allem anderen

Die Kette ist drei Ebenen tief: `review-against-rules` → `senior-code-reviewer`
(`model: opus` in dessen eigener Frontmatter) → ruft selbst
`/thermo-nuclear-code-quality-review` auf. Deren Rubriken decken laut eigenem Wortlaut ab:

| Zitat | deckt ab |
|---|---|
| „Ziehe bewusst Design Patterns und **Fowlers Refactoring-/Code-Smell-Katalog** heran" | grundsätzlich alle 23 Verifier |
| „Summentypen statt Flag-plus-optionale-Felder; illegale Zustände unkonstruierbar" | `illegal-state-check` |
| „Value Objects statt Primitives (*parse, don't validate*)" | `verifier-primitive-obsession` |
| „Abstraktions-Jagd … Pipeline, State Machine, Strategy, Policy, Summentyp" | `design-pattern-fit-check` |
| „Schichtung / Dependency-Regel … jede Kante ein Blocker" | `solid-principles-check` (DIP/SRP) |
| „Lies vorhandene Projektdokumentation (README, **ADRs**)" | `architecture-adr-check` (ADR-Hälfte) |
| thermo: „match expression", „declarative collection pipeline", „record syntax" | `language-idiom-check` |
| thermo: „file crossing `file_size_warn_lines`" | die Größen-Vorläufe von `large-class`/`long-method` |
| thermo: „keine Pass-Through-Wrapper, keine Zeremonie-Interfaces" | `middle-man`, `speculative-generality`, `lazy-class` |

Ein Leg von 31 überdeckte damit inhaltlich ~28 der übrigen 30 — und lief dabei auf Opus.

### 2. Die verbleibende Überlappung klumpt in Clustern

Die Skills wissen voneinander (fast jede Rubrik enthält ein „cross-reference rather than
double-counting"), aber die Überschneidung ist nicht paarweise:

| Cluster | Beteiligte | Eigenbeitrag |
|---|---|---|
| **A** Summentyp statt Flag+Optionals | `illegal-state`, `primitive-obsession`, `switch-statements`, `language-idiom` | `primitive-obsession`: soll der *Typ* existieren; `illegal-state`: ist der *Zustand* darstellbar |
| **B** Klasse/Datei zu groß | `solid` (SRP+Size), `large-class` (300), `long-method` (100), `divergent-change` | drei Schwellen auf **einem** Skript (`_shared/file_size_check.py`) |
| **C** Verzweigung → Polymorphie | `switch-statements`, `design-pattern` (Strategy/State), `solid` (OCP), `shotgun-surgery` | vierfach besetzt |
| **D** Logik am falschen Ort | `data-class` (Callee), `feature-envy` (Caller), `inappropriate-intimacy`, `middle-man` | zwei Sichten auf **einen** Defekt |
| **E** Abstraktion ohne Existenzberechtigung | `speculative-generality`, `lazy-class`, `dead-code`, `middle-man` | ein Trait mit einer Impl feuert bei dreien — mit drei **unvereinbaren** Fixes |
| **F** Duplikation | `duplicate-code`, `shotgun-surgery`, `incomplete-library-class` | `incomplete-library-class` hat mit dem Seam-Verstoß einen echten Eigenbeitrag |
| **G** Parameter-Gruppen | `data-clumps` ↔ `long-parameter-list` | **sauberste Trennung im Feld** — „Gruppe wiederkehrend" vs. „eine lange Signatur" |

### 3. Die Duplikation beschädigte den Loop, nicht nur das Budget

- **`Findings:`-Summen waren keine Defekt-Zahlen.** Ein Trait mit einer Impl erzeugte bis zu
  vier Findings; die Plateau-Erkennung verglich aufgeblähte Summen mit aufgeblähten Summen.
- **Fixer arbeiteten gegeneinander.** Dieselbe Stelle ging sequenziell an verschiedene Fixer
  mit unvereinbaren Rezepten — `fixer-lazy-class` (Inline Class) gegen
  `fixer-speculative-generality` (Collapse Hierarchy) gegen `fixer-dead-code` (löschen).
  `fixer-long-method` warnt selbst „cross-check `comments-fixer` so it isn't applied twice" —
  nur sah kein Fixer den Report eines anderen.
- **Jeder Fixer fuhr `run-tests`.** Bei sechs Fixern lief die Suite sechsmal pro Iteration.

**Entscheidung:** die Breite der Feinraster-Checks behalten, die Opus-Kette streichen. Beide
Netze gleichzeitig zu fahren war die eigentliche Verschwendung.

## Warum `agent_model: sonnet`

Der entscheidende Punkt, den man leicht falsch herum sieht: **der dispatchte Subagent
*führt* den Skill aus, er relayed ihn nicht.** Ruft er `verifier-long-method` per Skill-Tool
auf, werden dessen Instruktionen in *seinen* Kontext geladen und **er** fällt das Urteil.
Kein Verifier und kein Fixer spawnt selbst einen Agenten (`grep` über alle 46: null
`subagent_type`).

Nach dem Streichen der acht allgemeinen Validatoren ist damit **jedes** verbleibende Leg
urteilslastig — die früheren Ausnahmen (`lint-and-format-check` als reiner Exit-Code-Relay,
`review-against-rules` als Dispatcher vor einem eigenen Opus-Agenten) sind nicht mehr dabei.

Haiku wurde deshalb verworfen: die Rubriken sind explizit urteilsbehaftet
(`verifier-long-method` sagt selbst, die Zeilenzahl sei „only a proxy", der Befund seien
*mixed levels of abstraction*), und jedes Fehlurteil wird vom gepaarten Fixer in echte
Code-Mutation übersetzt. Der Loop verstärkt Fehlurteile, statt sie zu dämpfen.

## Warum Wellen statt strikt sequenziell

Fixer mutieren den Arbeitsbaum. Ihre **Themengebiete** sind disjunkt, ihre **Dateien** nicht
— `long-method` und `comments` treffen regelmäßig dieselbe Methode. Parallel wäre ein Race,
strikt sequenziell wäre unnötig langsam.

`scripts/plan_iteration.py` löst das deterministisch: es liest die Fundstellen aus den
Tabellenzeilen der Reports (nur dort, damit Prosa-Erwähnungen nicht als Pfade durchgehen)
und packt die Dispatches greedy in Wellen, die intern dateidisjunkt sind. Eine Unit **ohne**
erkennbare Dateien bekommt eine eigene Welle — unbekannte Reichweite gilt als „könnte alles
anfassen".

## Warum der Orchestrator die Fixer versorgt

Ziel: der Fixer soll nur editieren. Zwei Randbedingungen formen die Umsetzung.

- **Read-before-Edit ist Harness-Zwang.** Ein Subagent kann keine Datei editieren, die er
  nicht in *seinem* Kontext gelesen hat. Vollständig lesefrei ist technisch unmöglich;
  erreichbar ist: keine Suche, kein Verifier-Lauf, kein Grep — exakte Fundstellen rein, ein
  Read der zu ändernden Datei, fertig.
- **Reihenfolge macht Daten schal.** Welle 2 arbeitet auf einem Baum, den Welle 1 geändert
  hat. Deshalb sammelt der Orchestrator die Auszüge **je Welle unmittelbar vor dem
  Dispatch**, nicht alle vorab.

Das Test-Gate zog damit in den Orchestrator: einmal pro Iteration statt einmal pro Fixer.
Es ist zugleich das einzige objektive Signal, das nach dem Streichen von `qa-check` und
`lint-and-format-check` im Loop übrig ist — deshalb beendet ein rotes Gate den Lauf und
benennt die Welle, die den Baum zuletzt angefasst hat.

## Repo- und Sprachneutralität

Alle 46 Verifier/Fixer sind frei von ADR-Nummern, `CLAUDE.md`-Zitaten, `.rules/`-Pfaden,
„this repo"-Bezügen, Rust-/C#-Spezifika und deutschen Domänenbeispielen — auch in den
Beispielzeilen der Report-Tabellen. Die Regel steht normativ in
`_shared/validator-contract.md`, damit künftige Checks gar nicht erst repo-spezifisch
werden.

Anlass war nicht nur Portabilität: fünf Rubriken verankerten ihr Urteil nachweislich in
Doku, die es hier **nicht gibt** — ein `CLAUDE.md`-Zitat „Default to writing no comments",
ein „`bool WhatIf` ist verboten", eine „ADR-0008 slice-form rule" mit `Ausgang`-Typ, ein
„this repo bans `InternalsVisibleTo` (ADR-0007)" (ein C#-Konzept) und eine falsche
ADR-Nummer für die TestApi-Konvention. Alles Reste aus einem C#-Vorgänger-Repo, die hier
gegen nichts prüften.

**Wertvolle Ausnahmen blieben erhalten, aber als Prinzip statt als Repo-Verweis.**
`verifier-middle-man` schützt jetzt „eine bewusste Architektur-Naht — Port/Adapter-Grenze,
Facade zur Isolation einer Abhängigkeit, Anti-Corruption-Layer" statt einer ADR-Nummer. Das
hält in jedem Repo und verhindert weiterhin, dass der Fixer eine gewollte Naht entfernt.

**Was ein neues Repo trotzdem liefern muss:** `run-tests/config.json` (das Test-Kommando —
`validate-fix-loop` kennt den Stack bewusst nicht) und die drei `file_size_warn_lines` in
`verifier-long-method` / `-large-class` / `-long-parameter-list`. Die Schwellen sind
Trigger, keine Urteile; ein guter Startwert liegt nahe der 75. Perzentile der Dateilängen.

## Umkonfigurieren

Alles in `config.json`, nichts in `SKILL.md`:

| Schlüssel | Wirkung |
|---|---|
| `agent_model` | Modell **jedes** dispatchten Subagenten (Verifier wie Fixer) |
| `max_iterations` | Validierungs-Durchläufe. **`1` heißt Diagnose ohne Fix** — die letzte erlaubte Iteration validiert, dispatcht aber nie mehr Fixer, weil keine Iteration mehr folgte, die den Erfolg bestätigen könnte. `3` erlaubt zwei Fixer-Runden. |
| `validators` + `fixer_map` | Ein neues Smell-Paar ist ein Eintrag in beiden — keine Änderung an `SKILL.md`. |
| `test_gate` | `enabled` plus `command`/`args`; zeigt auf `run-tests`, damit das Repo-Wissen dort bleibt. |

## Was bewusst nicht mehr im Loop ist

Die acht allgemeinen Validatoren und `apply-validator-findings` sind **nur ausgehängt, nicht
gelöscht** — einzeln bleiben sie aufrufbar und für einen gezielten Tiefen-Review weiterhin
sinnvoll, allen voran `review-against-rules`. Sie tragen allerdings noch die Repo- und
Sprachbezüge, von denen die 46 Loop-Skills befreit wurden; wer sie in ein anderes Repo
mitnimmt, muss sie zuerst genauso entkoppeln.

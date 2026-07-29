---
name: verbessere-text
description: Improve a short text or keywords into three presentable variants in a chosen tone (default friendly/freundlich), then let the user pick one (or supply their own wording) via AskUserQuestion, and hand back the final pick in a copy-ready code block. Keeps the source language (de→de, en→en, no translation) and its native umlauts, and emits strictly dash-free output through a deterministic normalizer. Use when the user wants to "verbessere diesen Text", "formuliere das in <Ton> (sachlich/freundlich/förmlich/locker)", "mach drei Varianten daraus", "polish this text", "give me three versions of this in a <tone> tone", or hands over rough keywords to be turned into a clean line.
arguments: One required, one optional positional input. 1) `text` (required) — the short source text or keywords to improve. If missing, ask for it in one line; never guess. 2) `ton` (optional) — the tonality to write in (e.g. sachlich, freundlich, förmlich, locker; any tone the user names — do not validate or restrict it). If omitted, default to `freundlich` without asking.
---

# Verbessere Text

Take a short text or loose keywords and hand back **three presentable variants**
in a tone the user names (or a friendly default), then let them pick one — or
type their own wording — and return the pick ready to copy. One responsibility:
**reformulate one short text into three tone variants**, the same way every time.

**What this skill does NOT do:** change what the text *says* (only its wording and
tone), translate it into another language, invent extra arguments, validate or
restrict which tones are allowed, flatten umlauts, or carry any other step. It
reformulates and stops.

## ⛔ Iron rules

- **Meaning is fixed.** Improve only the *formulation* and the *tone*. Never add,
  drop, or alter a claim the source text makes.
- **Language is fixed.** Output language = the language of `text` (de→de, en→en).
  Detect it; never translate.
- **Umlauts stay umlauts.** German variants keep native `ä/ö/ü/Ä/Ö/Ü/ß` as-is —
  never flatten to `ae/oe/ue/ss`. This skill produces user-facing prose (emails,
  messages), not repo docs/code, so the project's ASCII-doc convention does not
  apply here; it would silently corrupt what the user copies out.
- **Dash-free output.** Every variant — and any wording the user types themselves —
  passes through `scripts/strip_dashes.py` exactly once before it is shown or
  returned. The generator already writes dash-free; the script is the deterministic
  net, not a re-generate gate.
- **File input only, never inline.** Always write the text to a scratchpad file first
  and pass the path to `scripts/strip_dashes.py`. Never interpolate variant or
  user-typed text directly into a shell command (e.g. `printf '%s' "<text>" | ...`) —
  free text may contain `$(...)`/backticks that a shell would execute. This applies
  uniformly to every call, not just "quote-heavy" ones.

## 0. Resolve the two arguments

Read `text` and `ton` from the invocation `arguments`.

- Missing `text`? Ask in one line: *„Welchen Text bzw. welche Stichworte soll ich
  verbessern?“* and stop.
- Missing `ton`? Default to `freundlich` and continue — do not ask.

Never guess `text`; never skip the `ton` default.

## The producer — `erzeuge_varianten(ton, text, ausgeschlossen)`

A **single** function produces three presentable variants. **Both** triggers — the
first run and the user picking "Keine der genannten" — call exactly this; there is
no second generation path. `ausgeschlossen` is the list of variants already shown
and rejected in this session (empty on the first call).

1. **Detect** the language of `text` → `sprache`.
2. **Generate** three distinct variants of `text`, using `ton` and `sprache` as
   the generation inputs. Instruct the generator (yourself) to **write without any
   dashes**, to **keep native umlauts** (`ä/ö/ü/ß` etc., never flattened to
   `ae/oe/ue/ss`), to keep the meaning, and — if `ausgeschlossen` is non-empty — to
   write three variants clearly distinct from every entry in `ausgeschlossen`, not
   just cosmetically reworded ones. Output language = `sprache` — **do not
   translate**.
3. **Normalize** each variant: write it to a scratchpad file, then run

   ```bash
   uv run .claude/skills/verbessere-text/scripts/strip_dashes.py <file>
   ```

   Never interpolate the variant inline into the command (see the file-input iron
   rule above). The script needs Python 3.10+; its PEP 723 header lets `uv run`
   provision one even when the system `python3` is older. It runs **once per
   variant**, never loops, never rejects — it just strips any stray dash.

**Returns:** three dash-free variants in `sprache`.

## The consumer loop

Repeat until the user **chooses** a variant. Maintain `ausgeschlossen`, the list of
variants shown and rejected so far, across iterations (starts empty):

1. `varianten = erzeuge_varianten(ton, text, ausgeschlossen)`.
2. Present them with **AskUserQuestion** — one single-select question (header
   `Variante`), **four options**: the three variants, plus a fourth option labelled
   **„Keine der genannten“**. Put each variant's full text in the option (as the
   label, with the full wording mirrored into the option's `description` when it is
   long) so the user can read it before choosing.
3. Act on the selection:
   - **One of the three variants** → done. Output it as the final result.
   - **„Keine der genannten“** → append `varianten` to `ausgeschlossen`, go back to
     step 1 (fresh variants, *same* `ton`, now excluding everything rejected so far).
   - **Free text via the automatic "Other" field** → treat it as the user's own
     desired wording: write it to a scratchpad file, normalize it through
     `scripts/strip_dashes.py <file>` (never inline), and output the cleaned result
     as the final result.

## Final output

Print the chosen (or user-supplied) wording inside a fenced code block (plain
text, no language tag) so the harness renders a one-click copy icon on it — just
the text, dash-free, umlauts intact, in the source language. No commentary, no
alternatives, no recap of the rejected variants, nothing outside the code block.

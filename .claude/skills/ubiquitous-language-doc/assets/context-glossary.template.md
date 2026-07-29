<!--
  CONTEXT.md SKELETON + WORKED EXAMPLE for ubiquitous-language-doc.

  This file has two parts:
    PART 1 — the skeleton you fill in, written to the repo root as CONTEXT.md.
    PART 2 — a worked example, for STRUCTURE & TONE ONLY. Do NOT copy its terms,
             definitions, or domain into the output. Every word you write must be
             DERIVED FROM THE TARGET CODE, never from this example.

  The skeleton follows the same shape as grill-with-docs / deepen-module
  (see ../../grill-with-docs/CONTEXT-FORMAT.md — that is the canonical format;
  do not invent a parallel one). Translate the prose and the dialogue header into
  the repo's language (German → `## Beispieldialog`; English → `## Example
  dialogue`), but keep `## Language` in English even in a German doc — that is the
  established convention. Match whatever an existing CONTEXT.md already uses.

  The `_Invariant_:` line below is an OPTIONAL extension of that canonical format
  (CONTEXT-FORMAT.md usually folds invariants into the definition prose); use it
  only when the code enforces a rule worth surfacing on its own line.
-->

<!-- ============================ PART 1: SKELETON ============================ -->

# {Context name — what this package/module/library is called}

{One or two sentences: what this library/context IS. Then an explicit scope line:
what it deliberately is NOT / does not cover. Derive both from the code's actual
surface, not from ambition.}

## Language

<!-- One entry per CORE term that the code establishes. Definition = what it IS,
     not what it does. Add invariants the code enforces. The _Avoid_ line lists
     misleading or false synonyms a newcomer might reach for. Omit any term the
     code does not actually establish — silence is correct when the code is silent. -->

**{Term}**:
{Precise definition — what it is. One or two sentences.}
_Invariant_: {a rule the type/contract guarantees, if the code enforces one}
_Avoid_: {misleading or false synonyms — words that mean something else here}

**{Term}**:
{Precise definition.}
_Avoid_: {…}

## Example dialogue

<!-- A short developer ⇄ domain-expert exchange that resolves a TYPICAL
     misunderstanding at a CONCRETE call site in this codebase. It should make the
     boundary between two easily-confused terms click. Quote real names/calls from
     the target code. -->

> **Dev:** {a plausible misconception phrased as a question, naming a real call}
> **Expert:** {the correction that pins the term down, grounded in the code}

<!-- ===================== PART 2: WORKED EXAMPLE (DO NOT COPY) ===================== -->
<!--
  EVERYTHING BELOW IS A STRUCTURE & TONE MODEL ONLY.

  It documents a generic ordering domain on purpose — so its content can never be
  mistaken for any real target. Match its shape, its tightness, and its level of
  detail. Do NOT carry "Order", "Cancellation", or any word below into the output
  unless the TARGET code independently establishes that exact concept.
-->

<!--

# Ordering

The bounded context that receives and tracks customer orders. It owns order
lifecycle and line items. It is NOT billing (invoices, payment) and NOT
fulfilment (warehouse picking, shipping) — those are separate contexts.

## Language

**Order**:
A customer's request to buy one or more line items, tracked from placement to
completion.
_Invariant_: an Order always has at least one line item; an empty Order cannot exist.
_Avoid_: Purchase, Transaction, Cart

**Cancellation**:
Voiding an entire Order before it ships. Always whole-Order — there is no
line-level cancel.
_Avoid_: Refund (that is Billing), Removal (that is editing a line item)

**Customer**:
The person or organisation that places Orders. Identified by `CustomerId`.
_Avoid_: Client, Buyer, Account (Account is an auth concept, not a domain one)

## Example dialogue

> **Dev:** When the user removes the last item, do we `order.Cancel()`?
> **Expert:** No — removing items is editing the Order. `Cancel()` voids the whole
> Order and is irreversible. And you can't remove the last item anyway: an Order
> with zero line items is an illegal state, so the UI must offer Cancel instead.

-->

export const meta = {
  name: 'multi-agent-thermo-nuclear-review',
  description: 'Multi-agent fan-out of the thermo-nuclear code-quality review: per lens, finder_count diverse finders + 1 adversarial verifier; one highest-impact finding — or an explicit found:false — per lens',
  phases: [
    { title: 'Find', detail: 'finder_count diverse finders per lens, distinct angles' },
    { title: 'Verify', detail: 'adversarial verify + impact-rank; one winner or found:false per lens' },
  ],
}

// All inputs arrive via `args` (the sandbox has no filesystem access); prepare_args.py built them.
// The Workflow host hands `args` over as the raw JSON string — parse it once at the boundary so the rest
// of the script always works against a real object (and stays correct if a host ever passes an object).
const input = typeof args === 'string' ? JSON.parse(args) : args
const { scope, lenses, finder_count, guardrails, thermoStandardsPath, templates, finderSchema, verifierSchema } = input

// Single substitution engine: replace every {{key}} the caller supplies. Values not in the map are left as-is.
// split(sep).join(v) inserts v LITERALLY. replaceAll(str, v) would instead interpret $$, $&, $` and $' inside v
// as special replacement patterns — and v is often free-form agent prose (candidates, scope) that contains them,
// which would silently corrupt the prompt. split/join has no such interpretation.
const fill = (tpl, map) =>
  Object.entries(map).reduce((s, [k, v]) => s.split(`{{${k}}}`).join(v), tpl)

const HEADER = fill(templates.header, { scope, guardrails, thermoStandardsPath })

// Render the finder candidates into the numbered text block the verifier re-reads.
const renderCandidates = (findings) =>
  findings
    .map((f, i) => `(${i + 1}) [${f.severity}] ${f.title} @ ${f.file}:${f.line}\n    what: ${f.what}\n    fix: ${f.improvement}\n    why: ${f.rationale}`)
    .join('\n')

// Stage 1 — finder_count diverse finders for this lens (barrier within the lens only).
// Returns just the flat findings array; the pipeline threads the original lens into stage 2 itself.
const runFinders = (lens) => {
  const angles = Array.from({ length: finder_count }, (_, i) => lens.angles[i % lens.angles.length])
  return parallel(angles.map((angle, i) => () =>
    agent(
      fill(templates.finder, { header: HEADER, lens: lens.name, angle }),
      { label: `find:${lens.name}:${i + 1}`, phase: 'Find', schema: finderSchema }
    )
  )).then(rs => {
    const findings = rs.filter(Boolean).flatMap(r => r.findings)
    log(`${lens.name}: ${findings.length} Kandidat(en) von ${angles.length} Findern`)
    return findings
  })
}

// Stage 2 — adversarial verifier: re-reads cited file:line, kills false positives + guardrail collisions,
// and returns the DISCRIMINATED result under `outcome` — either the single winning finding
// (outcome.found:true) or the explicit "nothing real" state (outcome.found:false). The sum type is nested
// because Anthropic structured output rejects oneOf at the top level. Zero candidates short-circuits without
// ever spawning the (expensive) verifier, so the agent is never handed an empty premise to invent from.
const runVerifier = (findings, lens) => {
  if (findings.length === 0) {
    log(`${lens.name}: keine Kandidaten -> outcome.found:false ohne Verifier-Aufruf`)
    return { lens: lens.name, outcome: { found: false, rejected: [] } }
  }
  return agent(
    fill(templates.verifier, { header: HEADER, lens: lens.name, candidates: renderCandidates(findings) }),
    { label: `verify:${lens.name}`, phase: 'Verify', schema: verifierSchema, effort: 'high' }
  )
}

// Pipelined per lens: a lens verifies as soon as its finders are in, while other lenses still review.
const results = await pipeline(lenses, runFinders, runVerifier)

// filter(Boolean) now drops ONLY crashed verifiers (null); found:false is a real, retained outcome.
return results.filter(Boolean)

# Review Rubric

For reviewers approving marketplace submissions. Pairs with `listing-quality.md` (the publisher-facing standard). Two gates: a mechanical one enforced in code, and a human one enforced here.

The bar is not "is this safe to run" — validation and binary signing already cover that. The bar is "will a first-time, non-technical user understand and trust this." That trust is the product.

## Mechanical gate (enforce in `cli/src/validate.rs`)

These are objective and should fail validation before a human ever sees the submission, using the same pattern as the existing structural checks:

- [ ] `description` is present and at least 40 characters.
- [ ] `category` is one of the live consumer shelves (reject freeform tags like "productivity").
- [ ] Every `inputs[].label` is present and non-empty.
- [ ] No `inputs[].label` or `options[].label` contains: `{{`, `}}`, `→`, leading/trailing underscores, ALL_CAPS_SNAKE_CASE, or regex/glob characters (`*`, `^`, `$`, `\`).
- [ ] Any `select`/`radio` input has at least 2 options, each with a non-empty human `label`.
- [ ] The listing includes at least one example asset (sample output or screenshot reference).

A failure here returns a specific message and blocks submission.

## Human review checklist (judgment)

Run every item. One failure = send back with a reason.

- [ ] **Benefit-first.** Does the description answer "what do I get?" before "how it works?" Reject mechanism-first copy ("produces a structured summary").
- [ ] **Real example.** Is the included example concrete and representative — not a placeholder?
- [ ] **Option sets fit real users.** For every `select`/`radio`, could the target user place themselves in the list? Flag missing obvious categories (e.g., no field-services option on a home-services agent).
- [ ] **Plain-language config.** Read the input form as a non-technical user would see it. Any label that reads like a code field — even if it passed the mechanical check — gets sent back.
- [ ] **Name not confusable.** Is the name distinct from existing listings? If close, does the tagline clearly differentiate?
- [ ] **Right shelf.** Is the chosen category the one a user with this problem would actually browse?
- [ ] **Persona quality** (per `building-agents.md`). Not generic slop; has judgment rules and a silence condition if it runs on a schedule.

## Decisions

**Approve** — clears the mechanical gate and every human item. Approving signs the binaries, builds the `.napp`, and activates the listing.

**Reject with reason** — name the specific item and quote the offending line. Templates:

- "Description leads with mechanism. Rewrite as the outcome the user gets — see `listing-quality.md` §1."
- "Input label '…' reads like a code field. Use plain language a non-technical user understands — §2."
- "Option set for '…' has no option for [target user]. Add it — §2."
- "Name is close to existing listing '…'. Differentiate the name or the tagline — §3."
- "Category '…' is not a shelf users browse. Pick the problem-named shelf this solves — §4."

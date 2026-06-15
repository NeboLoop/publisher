# Listing Quality

How to write a listing that gets installed. This is separate from `building-agents.md` (which covers making your agent *run well*) — this covers making your agent *understood and chosen* by someone who has never met you and never will.

## Why this matters

A user decides whether to install in about the time it takes to read one line. The same agent, with a clear listing, reaches thousands of people. With a vague one, it sits unused next to a solution that explained itself better. Your reach depends on this more than on anything in your code.

The platform keeps the chrome consistent — your card, your category, your install button all look the same as everyone else's. What you control is the words. These are the rules for those words.

**Where these words live.** Three fields, three readers: the manifest `description` (frontmatter) is for the *LLM* trigger-matching; the short `description` (≤500 chars) is the *card* line; the **long description** is the detail-page "What it does" body. Put the long version in a `LISTING.md` file next to your manifest — `neboai publish` reads it and sets it for you (via MCP, set `longDescription` on the update action). The display **name** is Title Case ("Nebo Design"); the slug stays the lowercase id. Mechanics live in the main skill's *Marketplace Listing* section; this file is the standard for *what to write*.

## 1. Description: lead with the outcome

The reader is asking one question: *what does this do for me?* Answer it before anything else.

Formula: **[verb] + [outcome] + [ease or timeframe]**

Mechanism-first (rewrite these):
- "Researches a topic and produces a structured summary."
- "An agent that aggregates multiple sources to generate reports."

Outcome-first (do this):
- "Get a clean research brief on any topic in two minutes."
- "Turn a messy pile of quotes into one client-ready proposal."

A reader should never have to translate what you wrote into what they get. If they could read your description and answer "so what?", rewrite it as the answer to that question.

### Include one concrete example

Every listing must show one real example of what it produces — a sample output, a before/after, or a screenshot. "Produces a summary" tells the reader nothing; showing the summary tells them everything. This is the single biggest difference between a listing that converts and one that doesn't.

## 2. Input labels: write for a person, not a config file

Your `inputs` become a form the user fills in at install. Every `label` and every option is read by someone who has never seen your code.

Each input `label` must be plain language a non-technical user understands on sight.

Not allowed in a label or option label:
- `SNAKE_CASE` or `camelCase` (`AUTO_REPLY_SENDER_TYPES`)
- Template syntax (`{{inbox_label}}`), regex, or glob patterns
- Internal field names or developer jargon (`pattern → label`, `cron`, `webhook`)
- A bare 24-hour time field with no hint of the expected format

Reads like a database column (rewrite):
```json
{ "key": "biz_type", "label": "BUSINESS_TYPE", "type": "select",
  "options": [{"value":"saas","label":"SaaS"},{"value":"ecom","label":"E-Commerce"}] }
```
A pool builder cannot find themselves in that list, and the label reads like a field name.

Reads like a question (do this):
```json
{ "key": "business_type", "label": "What kind of business do you run?", "type": "select",
  "options": [
    {"value":"home_services","label":"Home & field services (pools, HVAC, landscaping)"},
    {"value":"retail","label":"Shop or storefront"},
    {"value":"professional","label":"Professional services (legal, accounting, consulting)"},
    {"value":"other","label":"Something else"}
  ]}
```

Rule for option sets: cover the real users you are targeting. If your agent is for field-services businesses, the list must contain one. An option set the user cannot place themselves in is a broken install, even if it validates cleanly.

## 3. Name and tagline: findable, not confusable

Your card shows your name, your publisher, and a one-line tagline. Pick a name that does not collide with an obvious existing solution, and write a tagline that says what makes yours different in plain terms. If two listings are both called "Researcher," the tagline is the only thing telling them apart — so make it do that work.

## 4. Category: file it where the user looks

Tag your solution with the shelf a user would browse to find it — the problem they have, not the kind of thing you built. Choose from the live consumer shelves, for example:

- Run your business
- Create content
- Find customers
- Manage money
- Get organized
- Communicate
- Learn & grow
- Research & decide
- Handle documents
- Build & connect

"productivity" is not a shelf a user browses. "Run your business" is. Choosing the right shelf is how the right person finds you.

## Two gates

Every submission passes two checks:

1. **Automatic** — structural rules that fail fast (see `review-rubric.md` → Mechanical gate). A missing description, a snake_case label, or a template variable in a label is rejected before review.
2. **Human review** — judgment a machine cannot make: is the description actually benefit-first, does the option set fit real users, is the name confusable. A reviewer approves or sends it back with a reason.

Clearing both is not a hurdle. It is what makes this a marketplace a user trusts — which is what makes your listing worth installing in the first place.

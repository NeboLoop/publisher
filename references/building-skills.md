# Building Skills

How to write skills that actually trigger reliably, teach the agent effectively, and provide real value.

## The Description Is Everything

The description determines whether your skill ever activates. It's the single most important line you'll write.

### Bad Descriptions

```yaml
description: Helps with PDFs.
```
- Too vague. When would an agent pick this over any other skill?

```yaml
description: A tool for managing projects and tasks in the workspace.
```
- Generic. Dozens of skills could match this.

### Good Descriptions

```yaml
description: Extract text and tables from PDF files, fill PDF forms, merge multiple PDFs, and split PDFs into pages. Use when the user mentions PDF, form filling, document extraction, or page manipulation.
```
- Says what it does AND when to use it
- Includes the trigger keywords users actually say
- Specific enough to not conflict with other skills

### The Formula

```
[What it does — capabilities] + [When to use — trigger conditions]
```

Always answer both questions in under 1024 characters.

## Triggers That Work

Triggers are case-insensitive substring matches. Think about what users actually type:

```yaml
triggers:
  - create a pitch deck
  - investor deck
  - fundraising presentation
  - series A deck
  - startup pitch
```

**Common mistakes:**
- Too few triggers — skill never fires
- Too broad triggers (single common words like "email") — skill fires constantly
- Formal language nobody uses — "initiate correspondence" instead of "send email"

**Test your triggers:** Ask yourself "if I wanted this skill, what would I type?" Write those exact phrases.

## Progressive Disclosure Done Right

### Level 1: Frontmatter (~100 tokens, always loaded)

Only `name` and `description`. Keep this tight. Every skill's metadata is in context simultaneously.

### Level 2: SKILL.md body (loaded on activation)

This is where your instructions live. Rules:

1. **Under 500 lines.** Every line is a recurring token cost once loaded.
2. **Lead with the action.** Don't explain why — explain what to do.
3. **Be imperative.** "Check the inbox" not "You should check the inbox."
4. **Include decision trees.** When should the agent do X vs Y?

### Level 3: References (loaded on demand)

Factor these out of the body:
- API documentation with full parameter lists
- Long examples (>20 lines)
- Edge case catalogs
- Template files

Reference them from the body:
```markdown
For the complete API schema, see [references/api.md](references/api.md).
```

## Writing Instructions That Work

### Do This

```markdown
## Creating a Report

1. Gather data from the last 7 days
2. Group by category
3. Calculate totals and deltas from previous period
4. Format as markdown table
5. Highlight anything that changed >20%

If no data exists, say "No activity in the last 7 days" — do not generate empty tables.
```

### Not This

```markdown
## Creating a Report

When the user wants a report, you should think about what data they might need.
Consider looking at recent data, perhaps the last week or so. You might want to
group things by category if that makes sense. Calculate some totals if appropriate.
Format it nicely for them.
```

The first version is actionable. The second is vague hand-waving that produces inconsistent results.

### Key Principles

1. **Specifics over abstractions.** "7 days" not "recent". "$50,000" not "a significant amount".
2. **Decision rules over judgment.** "If >20%, highlight" not "highlight important changes".
3. **Output format upfront.** Show the agent exactly what the result should look like.
4. **Error cases explicitly.** What to do when data is missing, the API fails, or the request is ambiguous.
5. **One skill, one job.** Don't combine "email drafting" and "email sending" — they have different risk profiles.

## Capabilities Declaration

Declare only what you actually need:

```yaml
capabilities: [vision, storage]
```

**Do not over-request.** Users see these at install time. A skill requesting `[storage, network, vision, python, browser, calendar, email]` looks suspicious. Request the minimum.

| Capability | When to declare |
|------------|----------------|
| `storage` | You persist data across sessions |
| `network` | You call external APIs |
| `vision` | You analyze images |
| `python` | Your scripts/ directory has .py files |
| `typescript` | Your scripts/ directory has .ts files |
| `calendar` | You read or write calendar events |
| `email` | You access the inbox or send mail |
| `browser` | You navigate web pages |
| `notification` | You send push notifications |

## Scripts That Work

Scripts in `scripts/` execute without being loaded into context. Design them to:

1. **Accept arguments via CLI.** `python3 scripts/extract.py --input doc.pdf --output text.md`
2. **Print structured output.** JSON on stdout, errors on stderr.
3. **Be self-contained.** No `pip install` at runtime. Bundle dependencies or use stdlib only.
4. **Handle errors gracefully.** Exit code 0 on success, non-zero on failure. Print what went wrong.
5. **Be idempotent.** Running twice with the same input should produce the same result.

```python
#!/usr/bin/env python3
"""Extract text from a PDF file."""
import sys
import json

def main():
    if len(sys.argv) < 2:
        print(json.dumps({"error": "Usage: extract.py <file.pdf>"}))
        sys.exit(1)

    path = sys.argv[1]
    try:
        text = extract(path)
        print(json.dumps({"text": text, "pages": len(text.split("[Page"))}))
    except Exception as e:
        print(json.dumps({"error": str(e)}), file=sys.stderr)
        sys.exit(1)

if __name__ == "__main__":
    main()
```

## Plugin Dependencies

If your skill requires a plugin's tools:

```yaml
metadata:
  requires:
    plugins:
      - gws
    skills:
      - gws-shared
```

This tells Nebo to auto-install the plugin when the skill is installed. Without this, the skill's instructions reference tools that don't exist.

## Testing Your Skill

Before publishing:

1. **Trigger test:** Start a conversation and say phrases from your triggers. Does it activate?
2. **Instruction test:** Activate the skill and give it a realistic task. Does it follow your steps?
3. **Edge case test:** What happens with empty data? Invalid input? Missing dependencies?
4. **Conflict test:** Do any other installed skills also match your triggers? Which wins?

## Anti-Patterns

| Anti-Pattern | Why It Fails | Fix |
|-------------|-------------|-----|
| Giant SKILL.md (1000+ lines) | Crushes token budget | Factor into references/ |
| Vague instructions ("do the right thing") | Inconsistent results | Be specific and imperative |
| No error handling guidance | Agent hallucinates on failure | Add "if X fails, do Y" |
| Single-word triggers | Fires for unrelated requests | Use multi-word phrases |
| Requesting all capabilities | Users won't install | Only request what you use |
| Markdown formatting in instructions | Wastes tokens on decoration | Plain, terse text |
| Duplicating tool schemas | Stale when API changes | Reference the tool, describe *when* to use it |

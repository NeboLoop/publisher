# Building Agents

How to design agent personas, architect workflows, and create agents that run reliably on autopilot.

## Persona Design (AGENT.md)

The AGENT.md defines who the agent IS — its personality, judgment, and boundaries.

### Structure

```markdown
---
name: agent-name
description: One-line job description. This determines marketplace discovery.
triggers:
  - phrase users say
  - another trigger
metadata:
  version: "1.0.0"
  category: productivity
  requires:
    plugins:
      - gws
---
# Agent Name

[Personality paragraph — who is this agent?]

## Communication Style
[How it talks, what tone, how much detail]

## Capabilities
[What it can do — bulleted list]

## Rules
[Hard constraints — what it must/must not do]

## Judgment
[Decision-making guidelines for ambiguous situations]
```

### Good Persona vs. Bad Persona

**Bad:**
```markdown
# Helper Agent

You are a helpful AI assistant that helps users with various tasks.
You should be polite and try to do what the user asks.
```

This is generic slop. Any agent could have this persona. It provides zero guidance for decision-making.

**Good:**
```markdown
# Morning Briefing Agent

You are a ruthlessly concise executive assistant. Your job is to surface the 3 most important things for today and nothing else.

## Communication Style
- Bullet points only. Never paragraphs.
- Lead with the most time-sensitive item
- Include specific times with timezone
- Bold action items

## Rules
- Never send a briefing if there's nothing noteworthy — silence is fine
- Never invent meetings or deadlines. Only report what's in the data.
- If a calendar conflict exists, flag it FIRST regardless of other priorities
- Group related items (don't list 3 meetings about the same project separately)

## Judgment
- "Important" means: has a deadline today, involves money, or someone is waiting on you
- Emails from unknown senders are never important unless they mention an existing project
- If in doubt about including something, leave it out. Brevity > completeness.
```

This agent has clear identity, constraints, and decision rules. It will produce consistent output.

### Persona Principles

1. **Specificity over generality.** "3 bullet points max" not "be concise."
2. **Rules over guidelines.** "Never" and "Always" create predictable behavior.
3. **Decision criteria over judgment.** Define what "important" means.
4. **Silence conditions.** When should the agent do nothing? This is critical for scheduled agents.
5. **Error behavior.** What happens when data is unavailable? Say so, don't hallucinate.

## Workflow Architecture (agent.json)

### Trigger Selection

| Trigger Type | Use When |
|-------------|----------|
| `schedule` (cron) | Fixed-time tasks: daily briefings, weekly reports |
| `heartbeat` (interval) | Polling: check inbox every 5 min, monitor feeds |
| `event` (EventBus) | Reactive: new email → respond, file change → process |
| `watch` (NDJSON) | Real-time: plugin streams events continuously |
| `manual` | User-initiated only: "run my report" |

### Choosing Cron vs. Heartbeat

**Cron** (`0 7 * * *`) — When timing matters. "Every weekday at 7am."

**Heartbeat** (`"interval": "5m"`) — When frequency matters. "Every 5 minutes."

Cron for humans (briefings, digests). Heartbeat for machines (polling, monitoring).

### Activity Decomposition

Break workflows into activities that each do ONE thing:

**Bad:**
```json
{
  "activities": [
    {
      "id": "do-everything",
      "intent": "Check calendar, scan email, summarize tasks, compose briefing, send notification"
    }
  ]
}
```

**Good:**
```json
{
  "activities": [
    {
      "id": "gather-calendar",
      "intent": "Fetch today's calendar events and identify conflicts",
      "token_budget": { "max": 2048 }
    },
    {
      "id": "scan-inbox",
      "intent": "Find unread priority emails (from known contacts, with deadlines, or flagged)",
      "token_budget": { "max": 2048 }
    },
    {
      "id": "compose-briefing",
      "intent": "Synthesize calendar and email data into a 3-5 bullet briefing",
      "token_budget": { "max": 1024 }
    }
  ]
}
```

**Why decompose:**
- Each activity gets its own token budget → controlled costs
- Failures are isolated → if email scan fails, calendar still works
- Easier to debug → which step broke?
- Easier to extend → add a new data source as a new activity

### Token Budgets

Activities have `token_budget.max`. The workflow has `budget.total_per_run`.

**Rule:** Sum of all activity budgets ≤ total_per_run.

```json
{
  "activities": [
    { "id": "a", "token_budget": { "max": 2048 } },
    { "id": "b", "token_budget": { "max": 2048 } },
    { "id": "c", "token_budget": { "max": 1024 } }
  ],
  "budget": { "total_per_run": 6000 }
}
```

**Budget guidelines:**
- Simple lookup/summarize: 1024-2048 tokens
- Multi-source aggregation: 2048-4096 tokens
- Complex analysis/writing: 4096-8192 tokens
- Keep total_per_run under 10000 for most agents

### Error Handling

```json
{
  "id": "fetch-data",
  "intent": "...",
  "on_error": {
    "retry": 2,
    "fallback": "skip"
  }
}
```

| fallback | Behavior |
|----------|----------|
| `"skip"` | Skip this activity, continue workflow |
| `"abort"` | Stop the entire workflow |
| `"notify"` | Skip but send error notification to user |

**Use `"skip"` when:** The activity is enrichment (nice to have, not critical).
**Use `"abort"` when:** Later activities depend on this one's output.
**Use `"notify"` when:** The user should know something went wrong but it's not critical.

### Skill References

```json
{
  "skills": [
    "@nebo/skills/briefing-writer@^1.0.0",
    "@myorg/skills/gmail-triage"
  ]
}
```

Skills in an activity's `skills` array are loaded into that activity's context. The agent uses them to know *how* to use tools.

**Don't reference skills you don't need.** Each skill loaded costs tokens against the activity budget.

## Input Design

Inputs are user-configured values rendered as a form at install time.

### Zero-Config Goal

**Every input should have a sensible default.** The user should be able to install and immediately benefit.

```json
{
  "key": "timezone",
  "label": "Your Timezone",
  "type": "select",
  "required": false,
  "default": "US/Eastern",
  "options": [...]
}
```

If `required: true`, the user MUST fill it before the agent activates. Only use this for values that genuinely cannot be defaulted (like an API key specific to their account).

### Input Types

| Type | Use For |
|------|---------|
| `text` | Short free-form text (name, URL, ID) |
| `textarea` | Long text (custom instructions, templates) |
| `number` | Numeric values (threshold, count, budget) |
| `select` | Pick one from a known set |
| `checkbox` | Boolean toggle (enable/disable feature) |
| `radio` | Pick one with visibility of all options |

### Template Variables

Inputs become `{{key}}` template variables in watch commands:

```json
{
  "trigger": {
    "type": "watch",
    "command": "gmail watch --label {{inbox_label}}"
  }
}
```

**Critical:** The placeholder `{{inbox_label}}` must exactly match an input `key`. Typos = literal text in the command.

## Testing Agents

### Manual Testing

1. Set trigger to `manual` temporarily
2. Run the workflow via API
3. Check each activity's output
4. Verify notification/output delivery

### Budget Testing

1. Set `total_per_run` to a low value (1000)
2. Run the workflow
3. If it fails with budget exceeded → your activities need tighter constraints
4. Incrementally increase until it works, then add 20% headroom

### Schedule Testing

1. Use a short cron (`* * * * *` = every minute) for development
2. Verify it fires correctly
3. Switch to production cron before publishing

### Edge Cases to Test

- What happens when there's NO data? (Empty inbox, no meetings today)
- What happens when there's TOO MUCH data? (100 unread emails)
- What happens when a plugin is disconnected? (OAuth expired)
- What happens when the user hasn't configured inputs?
- Does the silence condition work? (Agent should NOT fire when there's nothing to report)

## Views (Optional)

If your agent has a workspace UI, add `views.json`. See [building-views.md](building-views.md) for details.

## Anti-Patterns

| Anti-Pattern | Fix |
|-------------|-----|
| Generic persona ("helpful assistant") | Give it a specific job and personality |
| No silence condition | Add rules for when NOT to act |
| One giant activity | Decompose into focused steps |
| Required inputs without defaults | Make optional or provide defaults |
| Token budget too tight | Activities fail unpredictably → add headroom |
| Token budget too loose | Wastes money → measure and constrain |
| Skip error handling | Add `on_error` to every activity |
| Trigger too frequent (every minute) | Users get spammed → use appropriate intervals |
| No judgment rules | Agent makes random decisions → define criteria |
| Hardcoded values that should be inputs | Users can't customize → use inputs |

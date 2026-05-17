---
name: meeting-prep
description: Prepare a briefing document before calendar meetings. Use when the user asks to prepare for a meeting, review upcoming meetings, or create pre-meeting summaries.
capabilities: [calendar, storage]
triggers:
  - prepare for meeting
  - meeting prep
  - briefing
  - what's my next meeting
metadata:
  version: "1.0.0"
  category: productivity
---
# Meeting Prep

Before each meeting, gather context and create a 1-page briefing.

## Steps

1. Check calendar for the next upcoming meeting
2. Identify attendees and their roles
3. Review any previous meeting notes (check storage)
4. Summarize the agenda and key discussion points
5. List any action items from previous meetings with these attendees

## Output Format

```
# Meeting Briefing: [Title]
**When:** [Date/Time]
**With:** [Attendees]

## Context
- [Previous interactions summary]

## Agenda
- [Key topics]

## Open Items
- [Pending action items]
```

## Notes

- Keep briefings under 1 page
- Focus on actionable context, not history
- Flag any conflicts or prep work needed

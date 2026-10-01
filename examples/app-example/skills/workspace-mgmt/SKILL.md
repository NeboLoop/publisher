---
name: workspace-mgmt
description: Manage deals, documents, and pipeline stages in the Deal Tracker app.
triggers:
  - create deal
  - list deals
  - move deal
  - upload document
  - deal pipeline
---
# Workspace Management

How to help with the deal pipeline the Deal Tracker page shows.

## What you can see

The page keeps the pipeline in the app's storage and shares it with you as
chat context:

- `pipeline`: every deal, each with `id`, `name`, `amount`, `stage`
  (`prospect`, `analysis`, `negotiation`, `closed`) and `created_at`.
- `deal`: the deal the user just opened, when they opened one.

Answer from that context. Never invent a deal, an amount or a stage that
is not in it.

## Listing deals

Group by stage in pipeline order (prospect → analysis → negotiation →
closed), with each deal's name and amount. Filter to one stage when the
user names it.

## Creating and moving deals

The page owns the pipeline: the user adds a deal with **+ New Deal** and the
page saves it. When the user asks you to create or move a deal, tell them
the exact name, amount and stage to enter, and confirm the amount and name
before they save.

## Analyzing a deal

When `deal` is in the context, analyze that deal: compute the metrics the
user asks for (cap rate, cash-on-cash, IRR), show your math, and flag
assumptions explicitly.

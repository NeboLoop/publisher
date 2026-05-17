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

Tools for managing the deal pipeline, documents, and analysis.

## list_deals

List all deals, optionally filtered by stage.
- **Method:** GET /deals
- **Query:** `stage` (optional) — filter by pipeline stage
- Returns: Array of deal objects with id, name, amount, stage, created_at

## create_deal

Create a new deal in the pipeline.
- **Method:** POST /deals
- **name** (string, required): Deal/property name
- **amount** (number, required): Deal value
- **stage** (string, optional): Initial stage (default: "prospect")
- Returns: Created deal object

## update_deal

Update a deal's stage or details.
- **Method:** PUT /deals/{id}
- **stage** (string, optional): Move to a new pipeline stage
- **name** (string, optional): Update deal name
- **amount** (number, optional): Update deal amount
- Returns: Updated deal object

## get_deal

Get full details for a specific deal.
- **Method:** GET /deals/{id}
- Returns: Deal object with all fields and attached documents

## delete_deal

Remove a deal from the pipeline.
- **Method:** DELETE /deals/{id}
- Returns: 204 No Content

## Workflow

When the user asks to move a deal, update its `stage` field:
- prospect → analysis → negotiation → closed

When creating a deal, always confirm the amount and name before saving.

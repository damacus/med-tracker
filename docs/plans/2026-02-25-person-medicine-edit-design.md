# Person Medicine Edit & Update Design

**Date:** 2026-02-25
**Issue:** #701 / med-tracker-zlx
**Branch:** claude/701

## Problem

`PersonMedicinesController` has `new/create/destroy/reorder/take_medicine` but no `edit/update` actions, despite routes exposing them. Users cannot edit existing person medicine records (notes, timing restrictions, max doses).

## Approach

Extend existing Modal/FormView/FormFields components with an `editing:` boolean flag. No new component files needed.

## Controller Changes

Add `edit` and `update` to `PersonMedicinesController`. Both actions use `set_person_medicine` (already used by destroy/reorder/take_medicine) and `authorize @person_medicine` (policy already defines `update?`).

- `edit` — responds HTML and Turbo Stream; renders modal or full page with `editing: true`
- `update` — responds HTML and Turbo Stream; on success removes modal and replaces card via `dom_id`; on failure re-renders modal/page with errors and `status: :unprocessable_entity`
- `person_medicine_params` — unchanged; `medicine_id` intentionally excluded (not editable)

## Component Changes

Three components gain an `editing: false` parameter:

- **`FormFields`** — medicine `<select>` renders with `disabled: @editing`; all other fields unchanged
- **`Modal`** — passes `editing:` to FormFields; form action uses `person_person_medicine_path` with `method: :patch` when editing; title becomes "Edit medicine"
- **`FormView`** — same as Modal

## Card UI

Add "Edit" button to `PersonMedicines::Card`, rendered only when `policy(person_medicine).update?`. Uses `link_to edit_person_person_medicine_path` with `data: { turbo_stream: true }` to trigger the modal inline.

## Authorization

No policy changes needed. `update?` is already defined:
- Admin allowed
- Self/dependent allowed
- Parent with minor allowed
- Carers: NOT allowed (can take but not manage)

## Testing

- **Request spec** — `PATCH /people/:person_id/person_medicines/:id`: success, validation failure, unauthorized
- **System spec** — open edit modal from person profile, update fields, see card reflect changes
- **Regression** — existing delete and reorder specs continue passing without modification

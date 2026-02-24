---
description: Create a PR title and description for a GitHub PR
auto_execution_mode: 2
---

- Summarise the current work
- If all work is pushed to GitHub, give me a pr title and description that I can paste into GitHub
- Otherwise ask what to do, give me 3 options
- Give me a squash commit message without a title or markdown formatting, use conventional commit messages
  - add feat:, fix: chore: to the body of the message if useful
  - Add scope to the message e.g. fix(medicines):

```bad example
Add stock forecast and refill inventory features

- Implement stock forecasting from active prescriptions with daily consumption calculation
- Add forecast UI showing days until low/out of stock on medicine detail page
- Create refill inventory modal with PaperTrail event tracking
- Fix reorder_threshold missing from strong parameters
- Add dosage and prescription factories for testing
- Fix MedicationTake versioning spec scope
```

```good example
feat(medicines): add stock forecast and refill inventory features

- feat: Implement stock forecasting from active prescriptions with daily consumption calculation
- feat: Add forecast UI showing days until low/out of stock on medicine detail page
- feat: Create refill inventory modal with PaperTrail event tracking
- fix: Fix reorder_threshold missing from strong parameters
- fix: Add dosage and prescription factories for testing
- fix: Fix MedicationTake versioning spec scope
```

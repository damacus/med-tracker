---
trigger: model_decision
description: When writing database queries, ActiveRecord code, or view components that access data
---

# Performance and Query Guidelines

## N+1 Query Prevention

- Never execute database queries inside view components or loops
- Use `includes`, `preload`, or `eager_load` in controllers for associations accessed in views
- Filter eager-loaded collections in-memory using `select`/`reject` instead of additional queries
- When passing data to components, pass pre-loaded associations

## Query Deduplication

- Extract repeated queries into variables or memoized methods
- Pass query results as parameters rather than re-querying in nested methods
- Use `size` on loaded collections instead of `count` (which triggers new query)

## Database vs In-Memory Operations

- Use `.where()` instead of Ruby's `select` for filtering ActiveRecord collections
- Use `.order()` instead of `sort_by` when data comes from the database
- Chain `.where().order()` to let the database handle both filtering and sorting
- Only use in-memory filtering (`select`/`reject`) on already-loaded/eager-loaded collections

## Existence Checks

- Before `create!`, check if record already exists to provide user-friendly errors
- Use guard clauses for early validation before transactions
- Prefer `exists?` over `find_by` when only checking existence

## Transaction Safety

- Wrap related creates/updates in `ActiveRecord::Base.transaction`
- Use `save!` inside transactions for consistent error handling
- Extract complex transaction logic into service objects

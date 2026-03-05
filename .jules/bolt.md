## 2025-03-02 - Use Enumerable Methods to Prevent N+1 Queries on Preloaded Associations

**Learning:** When displaying collections in views (like a dashboard schedule list), calling `association.where(...).count` or `association.order(...).first` on child objects forces ActiveRecord to issue a new query to the database for every single iteration, even if the association itself was preloaded with `.includes(:association)`. This introduces a severe N+1 query bottleneck.

**Action:** Whenever iterating over preloaded associations, use Ruby Enumerable methods (e.g., `association.count { |item| ... }`, `association.select { |item| ... }.max_by(&:field)`) instead of database-level queries (`.where.count` or `.order.first`). This allows Rails to perform filtering and sorting entirely in-memory using the already-loaded collection.

## 2025-03-05 - ActiveRecord .any? and loaded associations
**Learning:** ActiveRecord's `.any?` efficiently uses the loaded array or does an EXISTS query under the hood automatically. But if you chain a scope like `.active` to an association (e.g. `person.schedules.active.any?`), it will *always* hit the database, even if `person.schedules` is eagerly loaded, causing an N+1 query.
**Action:** When working with eagerly loaded associations, if you need to filter them, evaluate them in-memory using Enumerable methods like `.select` and then use `.present?` and `.size` on the resulting array to avoid database queries. Make sure the ruby logic matches the database scope correctly (especially handling `nil` vs SQL `NULL`).

## 2025-03-02 - Use Enumerable Methods to Prevent N+1 Queries on Preloaded Associations

**Learning:** When displaying collections in views (like a dashboard schedule list), calling `association.where(...).count` or `association.order(...).first` on child objects forces ActiveRecord to issue a new query to the database for every single iteration, even if the association itself was preloaded with `.includes(:association)`. This introduces a severe N+1 query bottleneck.

**Action:** Whenever iterating over preloaded associations, use Ruby Enumerable methods (e.g., `association.count { |item| ... }`, `association.select { |item| ... }.max_by(&:field)`) instead of database-level queries (`.where.count` or `.order.first`). This allows Rails to perform filtering and sorting entirely in-memory using the already-loaded collection.

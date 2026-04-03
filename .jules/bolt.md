## 2025-03-02 - Use Enumerable Methods to Prevent N+1 Queries on Preloaded Associations

**Learning:** When displaying collections in views (like a dashboard schedule list), calling `association.where(...).count` or `association.order(...).first` on child objects forces ActiveRecord to issue a new query to the database for every single iteration, even if the association itself was preloaded with `.includes(:association)`. This introduces a severe N+1 query bottleneck.

**Action:** Whenever iterating over preloaded associations, use Ruby Enumerable methods (e.g., `association.count { |item| ... }`, `association.select { |item| ... }.max_by(&:field)`) instead of database-level queries (`.where.count` or `.order.first`). This allows Rails to perform filtering and sorting entirely in-memory using the already-loaded collection.

## 2025-03-05 - ActiveRecord .any? and loaded associations
**Learning:** ActiveRecord's `.any?` efficiently uses the loaded array or does an EXISTS query under the hood automatically. But if you chain a scope like `.active` to an association (e.g. `person.schedules.active.any?`), it will *always* hit the database, even if `person.schedules` is eagerly loaded, causing an N+1 query.
**Action:** When working with eagerly loaded associations, if you need to filter them, evaluate them in-memory using Enumerable methods like `.select` and then use `.present?` and `.size` on the resulting array to avoid database queries. Make sure the ruby logic matches the database scope correctly (especially handling `nil` vs SQL `NULL`).

## 2026-03-05 - Manually Preloading Associations in Query Objects
**Learning:** When a query object (like `ScheduleQuery`) preloads records into a hash or separate collection, child models with custom logic (like `TimingRestrictions`) will still hit the database if they access the association and don't know it's already "preloaded". Even if you have the data, the model's `association.loaded?` check will return false.
**Action:** In query objects that perform bulk fetching, manually associate the child records with their parents using `source.association(:association_name).loaded!` and `source.association(:association_name).target.concat(preloaded_records)`. This ensures that downstream logic using the association (like `.count { ... }` on the loaded collection) avoids triggering new SQL queries.
## 2026-03-05 - Materialize Shared Card Data Once Per Render
**Learning:** In MedTracker card components, showing both a "today's doses" badge and the corresponding take list can accidentally trigger duplicate `medication_takes` work in the same render: one query for the count and another for the rows, plus repeated boolean checks if memoization uses `||=` for falsey values.
**Action:** When a card needs the same association for multiple UI elements, materialize it once into an array and reuse it for counts and rendering. For per-render boolean/nil memoization, prefer `instance_variable_defined?` guards over `||=` so blocked/out-of-stock states cache correctly.

## 2025-03-05 - Avoid N+1 Queries on `active_schedules`
**Learning:** In the DashboardPresenter, `active_schedules` was being populated with `.where(person_id: people.select(:id))` which resulted in a subquery, and then the views were calling `.count` or `.take(3)` directly on the relation which forced multiple queries.
**Action:** When accessing a collection multiple times across views (like `.count`, `.take(3)`, etc.), materialize it once in the presenter using `.load` or `.to_a`. Change the subquery to an in-memory map (`people.map(&:id)`) if the parent collection is already loaded, and always use enumerable methods (e.g., `.size`, `.to_a.take()`) in the views to prevent repeated database hits.

## 2025-03-05 - Memoize Computed Methods to Avoid Redundant SQL
**Learning:** In ActiveRecord models, computed methods like `estimated_daily_consumption` that iterate over associations (e.g. `schedules.sum`) trigger database calls or array aggregations. When these computed methods are called multiple times in the same view (like inside `forecast_available?`, `days_until_low_stock`, etc.), it causes redundant queries.
**Action:** Use instance variable memoization (`return @variable if defined?(@variable)`) for computed methods that process associations but don't change state during a render cycle. This drastically reduces redundant Ruby object allocation and database interactions when the view relies on those computations.

## 2026-03-08 - Use Enumerable#find Instead of ActiveRecord#find_by on Preloaded Associations
**Learning:** ActiveRecord's `find_by` always hits the database even if the association is preloaded via `.includes()` or `.eager_load()`. Calling `.find_by(attribute: value)` inside an iteration (e.g. `default_dosage_for_person_type` rendering multiple medications) triggers an N+1 query bottleneck.
**Action:** When finding a specific record within an eagerly loaded association, materialize the collection using `.to_a` (or if it's already an array) and use Ruby's `Enumerable#find` (e.g., `association.to_a.find(&:attribute?)`) to evaluate the filter in-memory and avoid subsequent SQL queries.

## 2026-03-07 - Cache Timing Restriction State Per Card Render
**Learning:** MedTracker medication cards can evaluate the same timing restriction logic multiple times in one render path: once for countdown visibility, again for the disabled button state, and again for disabled-label copy. On cooldown states this multiplies `can_take_now?` work across every visible card.
**Action:** In card components that render medication actions, cache `can_take_now?`, out-of-stock state, and blocked reason per render using instance variables guarded with `instance_variable_defined?`. That keeps cooldown/out-of-stock renders to a single timing evaluation while preserving behavior.

## 2026-03-08 - Avoid Redundant Subqueries When Array is Materialized Immediately Afterwards
**Learning:** In `reports_controller.rb`, extracting `Schedule.where(person_id: person_ids).select(:id)` as a subquery constraint for `MedicationTake` forced an extra database subquery, even though the exact filtered subset of active `schedules` was materialized into an array on the very next line via `.to_a`.
**Action:** Order ActiveRecord relation evaluations logically to maximize reuse. If a collection must be materialized into a Ruby array anyway, compute it first, and then map its IDs (`schedules.map(&:id)`) to constrain subsequent queries instead of duplicating the database effort with a subquery.

## 2025-03-09 - Avoid O(N^2) Linear Searches in Nested Loops
**Learning:** In view components (like `Locations::ShowView`), looping over a collection (e.g. `location.members`) and performing a linear search (`.find { ... }`) on another association inside the loop creates an O(N^2) performance bottleneck, especially if both collections can grow.
**Action:** Before entering the loop, pre-index the association being searched using `.index_by(&:foreign_key)`. Then, perform O(1) hash lookups inside the loop (e.g. `indexed_hash[item.id]`), effectively reducing the operation to O(N).

## 2025-03-02 - Memoize Derived Database Properties

**Learning:** Computations like `estimated_daily_consumption` in `Medication` iterate over collections (e.g. `schedules` and `person_medications`). Since properties like `forecast_available?`, `days_until_out_of_stock`, and `days_until_low_stock` reference `estimated_daily_consumption` repeatedly during view renders, this leads to redundant iteration and query-like evaluation loops on the model layer.

**Action:** Whenever a model calculation involves looping over multiple records and is referenced multiple times per render cycle, use memoization (e.g., `@var ||= begin ... end`) to store the calculation outcome.

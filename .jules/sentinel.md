## 2024-03-09 - Insecure Direct Object Reference (IDOR) via Direct Model Find
**Vulnerability:** Controller actions using `Model.find(params[:id])` directly without scoping to the current user's authorized records.
**Learning:** Even with Pundit `authorize @record` checks later, fetching records without `policy_scope` can expose existence of records and potentially lead to authorization bypasses if policies are misconfigured. It also triggers `ActiveRecord::RecordNotFound` instead of `Pundit::NotAuthorizedError`, standardizing 404 responses for unauthorized access, preventing information disclosure.
**Prevention:** Always use `policy_scope(Model).find(params[:id])` pattern in controller actions to enforce scope-based authorization at the database query level.

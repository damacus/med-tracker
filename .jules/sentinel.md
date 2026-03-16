
## 2026-03-16 - Fix IDOR in LocationMembershipsController and Admin::InvitationsController
**Vulnerability:** Insecure Direct Object Reference (IDOR) due to missing policy scope when retrieving records by ID.
**Learning:** Even when the policy checks admin authorization, it's best practice to always scope the queries using `policy_scope(Model)` for defense-in-depth and consistency across the application.
**Prevention:** Always use `policy_scope(Model).find(params[:id])` instead of `Model.find(params[:id])` in controller actions.

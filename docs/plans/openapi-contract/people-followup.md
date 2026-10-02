# Profile and person writes: next source pointers

After dosage completion, the original 89-operation baseline contains
`getCurrentProfile`, `createPerson`, `updatePerson` and `updatePersonWithPut`.
The existing people read routes can support verification but are not new
baseline completions. Document missing rules in OpenAPI before deriving tests.

- `MeController#show` returns the current user's person and account, plus the
  selected membership role. Use `MeSerializer` and `PersonSerializer` to check
  exact output. A user may select a different household from their own person's
  household; do not substitute a different person's medical profile.
- `PersonPolicy` permits creation by household managers or members holding a
  manage grant. Updates require manage access to that person; visible-only
  records remain unwritable. The policy scope filters by person grants.
- `People::Create` performs a transaction. For minors/dependent adults with a
  membership-linked carer, it calls `CareDelegation::Assign`; otherwise it saves
  and creates a manage grant through `Households::AccessChange`. Inspect these
  services before implementing side effects or credential-version changes.
- `Person` normalizes email, requires name/date of birth, validates email and
  uniqueness, enforces capacity/carer and person-type/age rules, assigns a Home
  location on creation, and records audit and sync changes. Preserve enum
  `adult:0`, `minor:1`, `dependent_adult:2`; minors and dependent adults cannot
  retain capacity. Do not replace these effects with a bare table insert.
- Person PATCH/PUT currently have no If-Match parameter or 409 response in the
  specification, and the Rails update does not call `fresh_api_record?`. Do not
  invent mandatory conditional updates while implementing these operations.
- Household authentication currently has shared implementation debt around app
  tokens. Account-session support alone does not establish app-token support
  for household operations; assess the documented credential contract directly.

Sources: `app/controllers/api/v1/me_controller.rb`,
`app/controllers/api/v1/people_controller.rb`, `app/policies/person_policy.rb`,
`app/services/people/create.rb`, `app/models/person.rb`, and the corresponding
serializers under `app/serializers/api/v1/`.

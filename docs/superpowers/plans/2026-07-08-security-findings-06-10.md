# Security Findings 06-10 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remediate the next five open MedTracker security findings: delegated location imports, cross-household medication locations, owner demotion, owner deactivation, and profile email-change verification bypasses.

**Architecture:** Keep write authorization at the boundary before mutations run. Extend the portable import preflight created in the first remediation PR, enforce medication tenant integrity in the model and controller parameter boundary, share owner-account rules through one admin service, and make Rodauth change-login the only path that can change account email.

**Tech Stack:** Ruby 4.0.5, Rails 8.1.3, Pundit policies, Rodauth login-change verification, RSpec request/model/service specs, Rails fixtures, `task` commands.

---

## File Structure

- Modify `app/services/portable_data/importer.rb` to reject delegated `locations` rows before `PortableData::ImportWriter` runs.
- Modify `spec/services/portable_data/importer_spec.rb` to cover delegated location rejection and manager regressions.
- Modify `app/models/medication.rb` to validate that `location.household_id` matches `medication.household_id`.
- Modify `app/controllers/concerns/medication_form_context.rb` to resolve web-form `location_id` through `policy_scope(Location)`.
- Modify `app/controllers/api/v1/medications_controller.rb` to resolve API `location_id` through `policy_scope(Location)`.
- Modify `spec/models/medication_spec.rb` for model tenant validation.
- Modify `spec/requests/api/v1/medications_spec.rb` for API cross-household location rejection.
- Create `app/services/admin/owner_governance.rb` for owner role and usable-owner checks.
- Modify `app/services/admin/membership_role_updater.rb` to block owner demotion by non-owners.
- Modify `app/controllers/admin/users_controller.rb` to block unsafe owner deactivation.
- Modify `spec/requests/admin/user_mutation_boundary_spec.rb` for owner demotion and deactivation behavior.
- Modify `config/locales/en.yml` and matching locale files to add user-facing rejection messages.
- Modify `app/controllers/profiles_controller.rb` to reject direct profile email changes and redirect users to Rodauth change-login.
- Modify `spec/requests/profiles_show_spec.rb` to prove direct profile email mutation is blocked and Rodauth creates a verification-backed login change.

## Task 1: Reject Delegated Portable Location Writes

**Files:**
- Modify: `app/services/portable_data/importer.rb`
- Test: `spec/services/portable_data/importer_spec.rb`

- [ ] **Step 1: Write the failing delegated location import spec**

Add this example after the existing delegated medication and dosage authorization specs in `spec/services/portable_data/importer_spec.rb`:

```ruby
it 'rejects member location writes before the import writer mutates records' do
  household = create(:household)
  manageable_person = create(:person, household: household, portable_id: 'manageable-person-portable')
  location = create(:location, household: household, portable_id: 'location-portable', name: 'Original Shelf')
  membership = member_membership(household, person: manageable_person)
  grant_manage_access(household: household, membership: membership, person: manageable_person)
  payload = solo_person_payload_for(manageable_person)
  payload[:records][:locations] = [
    { portable_id: location.portable_id, name: 'Delegated Rename', description: 'Changed by import' }
  ]

  expect do
    import_result(household: household, membership: membership, payload: payload, dry_run: false)
  end.to raise_error(Pundit::NotAuthorizedError)

  expect(location.reload).to have_attributes(name: 'Original Shelf', description: nil)
end
```

- [ ] **Step 2: Write the manager regression spec**

Add this example in the same file:

```ruby
it 'keeps owner and administrator location imports unrestricted within the household' do
  household = create(:household)

  [owner_membership(household), administrator_membership(household)].each_with_index do |membership, index|
    location = create(
      :location,
      household: household,
      portable_id: "manager-location-portable-#{index}",
      name: "Manager Location #{index}"
    )
    payload = portable_payload.deep_dup
    payload[:records] = payload[:records].transform_values { [] }
    payload[:records][:locations] = [
      {
        portable_id: location.portable_id,
        name: "Manager Updated Location #{index}",
        description: "Manager description #{index}"
      }
    ]

    result = import_result(household: household, membership: membership, payload: payload, dry_run: false)

    expect(result).to be_applied
    expect(location.reload).to have_attributes(
      name: "Manager Updated Location #{index}",
      description: "Manager description #{index}"
    )
  end
end
```

- [ ] **Step 3: Run the focused spec and verify it fails**

Run: `task test TEST_FILE=spec/services/portable_data/importer_spec.rb`

Expected: the delegated location example fails because the member import still reaches `PortableData::ImportWriter#import_locations` and renames the location.

- [ ] **Step 4: Implement the importer preflight**

Change `app/services/portable_data/importer.rb` so `authorize_record_writes!` also checks location rows:

```ruby
def authorize_record_writes!(manageable_person_portable_ids)
  authorized_ids = medication_portable_ids_for(manageable_person_portable_ids)
  return if location_rows_authorized? &&
            medication_rows_authorized?(authorized_ids) &&
            dosage_rows_authorized?(authorized_ids)

  raise Pundit::NotAuthorizedError
end

def location_rows_authorized?
  records(:locations).empty?
end
```

- [ ] **Step 5: Run the focused spec and verify it passes**

Run: `task test TEST_FILE=spec/services/portable_data/importer_spec.rb`

Expected: PASS with all importer examples green.

- [ ] **Step 6: Commit**

```bash
git add app/services/portable_data/importer.rb spec/services/portable_data/importer_spec.rb
git commit -m "fix(security): reject delegated location imports"
```

## Task 2: Enforce Medication Location Household Boundaries

**Files:**
- Modify: `app/models/medication.rb`
- Modify: `app/controllers/concerns/medication_form_context.rb`
- Modify: `app/controllers/api/v1/medications_controller.rb`
- Test: `spec/models/medication_spec.rb`
- Test: `spec/requests/api/v1/medications_spec.rb`

- [ ] **Step 1: Write the failing model validation specs**

Add these examples inside `describe 'validations'` in `spec/models/medication_spec.rb`:

```ruby
it 'allows a location from the same household' do
  household = create(:household)
  location = create(:location, household: household)
  medication = build(:medication, household: household, location: location)

  expect(medication).to be_valid
end

it 'rejects a location from another household' do
  household = create(:household)
  other_location = create(:location, household: create(:household))
  medication = build(:medication, household: household, location: other_location)

  expect(medication).not_to be_valid
  expect(medication.errors[:location]).to include('must belong to the same household')
end
```

- [ ] **Step 2: Write the failing API boundary spec**

Add this example inside `describe 'PATCH /api/v1/households/:household_id/medications/:id'` in `spec/requests/api/v1/medications_spec.rb`:

```ruby
it 'rejects a location from another household' do
  login_data = api_login(user)
  household_id = login_data.dig('household', 'id')
  medication = medications(:paracetamol)
  original_location = medication.location
  other_household = Household.create!(name: 'Medication Location Boundary', slug: 'med-location-boundary')
  other_location = Location.create!(household: other_household, name: 'Other Household Cabinet')

  patch api_v1_household_medication_path(household_id, medication.id),
        params: { medication: { location_id: other_location.id } },
        headers: api_auth_headers(login_data.fetch('access_token')),
        as: :json

  expect(response).to have_http_status(:not_found)
  expect(medication.reload.location).to eq(original_location)
end
```

- [ ] **Step 3: Run focused specs and verify they fail**

Run: `task test TEST_FILE=spec/models/medication_spec.rb`

Expected: FAIL because `Medication` does not validate the location household.

Run: `task test TEST_FILE=spec/requests/api/v1/medications_spec.rb`

Expected: FAIL because the API accepts the raw `location_id` before the model guard exists.

- [ ] **Step 4: Add the model validation**

Modify `app/models/medication.rb`:

```ruby
validate :single_dose_switch_requires_no_schedules
validate :nested_dosage_records_are_valid
validate :location_must_belong_to_household
before_validation :assign_household
after_commit :sync_dosages, on: :update
```

Add this private method:

```ruby
def location_must_belong_to_household
  return if location.blank? || household.blank? || location.household_id == household_id

  errors.add(:location, 'must belong to the same household')
end
```

- [ ] **Step 5: Resolve web medication form locations through policy scope**

Modify `app/controllers/concerns/medication_form_context.rb`:

```ruby
def medication_params
  params.expect(
    medication: [
      :name,
      :friendly_name,
      :barcode,
      :dmd_code,
      :dmd_system,
      :dmd_concept_class,
      :category,
      :description,
      :dose_amount,
      :dose_unit,
      :current_supply,
      :reorder_threshold,
      :warnings,
      :location_id,
      :default_schedule_type,
      :default_schedule_config,
      { default_schedule_config: SCHEDULE_CONFIG_KEYS },
      { dosage_records_attributes: [%i[
        id
        amount
        unit
        frequency
        description
        default_for_adults
        default_for_children
        default_max_daily_doses
        default_min_hours_between_doses
        default_dose_cycle
        current_supply
        reorder_threshold
        _destroy
      ]] }
    ]
  ).tap do |permitted|
    MedicationParamsNormalizer.call(permitted, schedule_config_keys: SCHEDULE_CONFIG_KEYS)
    constrain_medication_location!(permitted)
  end
end

def constrain_medication_location!(permitted)
  location_id = permitted[:location_id].presence
  return if location_id.blank?

  permitted[:location_id] = policy_scope(Location).find(location_id).id
end
```

- [ ] **Step 6: Resolve API medication locations through policy scope**

Modify `app/controllers/api/v1/medications_controller.rb`:

```ruby
def medication_params
  params.expect(
    medication: %i[
      name
      friendly_name
      barcode
      dmd_code
      dmd_system
      dmd_concept_class
      category
      description
      dose_amount
      dose_unit
      current_supply
      reorder_threshold
      warnings
      location_id
      default_schedule_type
    ]
  ).tap { |permitted| constrain_medication_location!(permitted) }
end

def constrain_medication_location!(permitted)
  location_id = permitted[:location_id].presence
  return if location_id.blank?

  permitted[:location_id] = policy_scope(Location).find(location_id).id
end
```

- [ ] **Step 7: Run focused specs and verify they pass**

Run: `task test TEST_FILE=spec/models/medication_spec.rb`

Expected: PASS.

Run: `task test TEST_FILE=spec/requests/api/v1/medications_spec.rb`

Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add app/models/medication.rb app/controllers/concerns/medication_form_context.rb app/controllers/api/v1/medications_controller.rb spec/models/medication_spec.rb spec/requests/api/v1/medications_spec.rb
git commit -m "fix(security): enforce medication location household scope"
```

## Task 3: Block Household Administrators From Demoting Owners

**Files:**
- Create: `app/services/admin/owner_governance.rb`
- Modify: `app/services/admin/membership_role_updater.rb`
- Modify: `config/locales/en.yml`
- Modify: every supported locale file with matching key structure: `config/locales/cy.yml`, `config/locales/es.yml`, `config/locales/ga.yml`, `config/locales/pt.yml`
- Test: `spec/requests/admin/user_mutation_boundary_spec.rb`

- [ ] **Step 1: Write the failing administrator demotion spec**

Add this example to `spec/requests/admin/user_mutation_boundary_spec.rb` after the owner-promotion rejection spec:

```ruby
it 'rejects administrator attempts to demote existing owners' do
  owner = users(:parent)
  administrator = users(:jane)
  owner_membership = upsert_household_membership(owner.person, :owner)
  upsert_household_membership(administrator.person, :administrator)
  sign_in(administrator)

  patch membership_role_admin_user_path(owner), params: { membership: { role: 'member' } }

  expect(response).to redirect_to(admin_users_path)
  expect(flash[:alert]).to include('Only household owners can change owner memberships')
  expect(owner_membership.reload.role).to eq('owner')
end
```

- [ ] **Step 2: Write the owner regression spec**

Add this example in the same file:

```ruby
it 'allows a household owner to demote another owner when another owner remains' do
  target_owner = users(:parent)
  target_membership = upsert_household_membership(target_owner.person, :owner)

  patch membership_role_admin_user_path(target_owner), params: { membership: { role: 'member' } }

  expect(response).to redirect_to(admin_users_path)
  expect(target_membership.reload.role).to eq('member')
end
```

- [ ] **Step 3: Run the focused spec and verify it fails**

Run: `task test TEST_FILE=spec/requests/admin/user_mutation_boundary_spec.rb`

Expected: FAIL because administrators can currently demote owner memberships.

- [ ] **Step 4: Create the shared owner governance service**

Create `app/services/admin/owner_governance.rb`:

```ruby
# frozen_string_literal: true

module Admin
  class OwnerGovernance
    def initialize(household:, actor_membership:)
      @household = household
      @actor_membership = actor_membership
    end

    def can_change_owner_membership?(target_membership)
      !owner_membership?(target_membership) || actor_membership&.owner?
    end

    def can_deactivate_owner_user?(target_membership)
      return true unless owner_membership?(target_membership)

      actor_membership&.owner? && usable_owner_count_excluding(target_membership).positive?
    end

    private

    attr_reader :household, :actor_membership

    def owner_membership?(membership)
      membership&.owner? && membership&.active?
    end

    def usable_owner_count_excluding(target_membership)
      return 0 unless household && target_membership

      household.household_memberships.owner.active
               .where.not(id: target_membership.id)
               .joins(account: { person: :user })
               .where(users: { active: true })
               .count
    end
  end
end
```

- [ ] **Step 5: Use the governance service in membership role updates**

Modify `app/services/admin/membership_role_updater.rb`:

```ruby
def call
  return Result.new(false, I18n.t('admin.membership_roles.owner_rejected')) if role == OWNER_ROLE
  return Result.new(false, I18n.t('admin.membership_roles.invalid_role')) unless allowed_role?
  return Result.new(false, I18n.t('admin.membership_roles.owner_demotion_rejected')) unless owner_change_allowed?

  previous_role = membership.role
  ActiveRecord::Base.transaction do
    membership.update!(role: role)
    record_audit_event(previous_role)
  end
  Result.new(true, I18n.t('admin.membership_roles.updated'))
end
```

Add these private methods:

```ruby
def owner_change_allowed?
  owner_governance.can_change_owner_membership?(membership)
end

def owner_governance
  @owner_governance ||= OwnerGovernance.new(
    household: membership.household,
    actor_membership: actor_membership
  )
end
```

- [ ] **Step 6: Add locale keys**

Modify `config/locales/en.yml` under `admin.membership_roles`:

```yaml
      owner_demotion_rejected: Only household owners can change owner memberships.
```

Modify `config/locales/cy.yml` under `admin.membership_roles`:

```yaml
      owner_demotion_rejected: Dim ond perchnogion cartrefi sy'n gallu newid aelodaeth perchennog.
```

Modify `config/locales/es.yml` under `admin.membership_roles`:

```yaml
      owner_demotion_rejected: Solo los propietarios del hogar pueden cambiar las membresias de propietario.
```

Modify `config/locales/ga.yml` under `admin.membership_roles`:

```yaml
      owner_demotion_rejected: Ni feidir ach le huineiri teaghlaigh ballraiochtai uineara a athru.
```

Modify `config/locales/pt.yml` under `admin.membership_roles`:

```yaml
      owner_demotion_rejected: Apenas os proprietarios do agregado podem alterar associacoes de proprietario.
```

- [ ] **Step 7: Run the focused spec and verify it passes**

Run: `task test TEST_FILE=spec/requests/admin/user_mutation_boundary_spec.rb`

Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add app/services/admin/owner_governance.rb app/services/admin/membership_role_updater.rb config/locales/en.yml config/locales/cy.yml config/locales/es.yml config/locales/ga.yml config/locales/pt.yml spec/requests/admin/user_mutation_boundary_spec.rb
git commit -m "fix(security): require owner authority for owner demotion"
```

## Task 4: Block Unsafe Owner User Deactivation

**Files:**
- Modify: `app/services/admin/owner_governance.rb`
- Modify: `app/controllers/admin/users_controller.rb`
- Modify: `config/locales/en.yml`
- Modify: every supported locale file with matching key structure: `config/locales/cy.yml`, `config/locales/es.yml`, `config/locales/ga.yml`, `config/locales/pt.yml`
- Test: `spec/requests/admin/user_mutation_boundary_spec.rb`

- [ ] **Step 1: Write the failing administrator owner-deactivation spec**

Add this example to `spec/requests/admin/user_mutation_boundary_spec.rb`:

```ruby
it 'rejects administrator attempts to deactivate owner users' do
  owner = users(:parent)
  administrator = users(:jane)
  upsert_household_membership(owner.person, :owner)
  upsert_household_membership(administrator.person, :administrator)
  sign_in(administrator)

  delete admin_user_path(owner)

  expect(response).to redirect_to(admin_users_path)
  expect(flash[:alert]).to include('Owner accounts can only be deactivated by another household owner')
  expect(owner.reload.active?).to be(true)
end
```

- [ ] **Step 2: Write last-usable-owner and allowed-owner specs**

Add these examples in the same file:

```ruby
it 'rejects deactivation when it would leave no other active owner user' do
  target_owner = users(:parent)
  target_membership = upsert_household_membership(target_owner.person, :owner)
  admin_membership = household.household_memberships.find_by!(account: admin.person.account)
  admin.update!(active: false)

  delete admin_user_path(target_owner)

  expect(response).to redirect_to(admin_users_path)
  expect(flash[:alert]).to include('Owner accounts can only be deactivated by another household owner')
  expect(target_owner.reload.active?).to be(true)
  expect(target_membership.reload.role).to eq('owner')
  expect(admin_membership.reload.role).to eq('owner')
end

it 'allows an owner to deactivate another owner when another active owner user remains' do
  target_owner = users(:parent)
  upsert_household_membership(target_owner.person, :owner)

  delete admin_user_path(target_owner)

  expect(response).to redirect_to(admin_users_path)
  expect(target_owner.reload.active?).to be(false)
end
```

- [ ] **Step 3: Run the focused spec and verify it fails**

Run: `task test TEST_FILE=spec/requests/admin/user_mutation_boundary_spec.rb`

Expected: FAIL because `Admin::UsersController#destroy` only blocks self-deactivation.

- [ ] **Step 4: Add owner deactivation guard to the controller**

Modify `app/controllers/admin/users_controller.rb` inside `destroy`:

```ruby
if @user == current_user
  format.html { redirect_to admin_users_path, alert: t('users.cannot_deactivate_self') }
  format.turbo_stream do
    flash.now[:alert] = t('users.cannot_deactivate_self')
    render turbo_stream: turbo_stream.update('flash', Components::Layouts::Flash.new(alert: flash[:alert])),
           status: :unprocessable_content
  end
elsif owner_deactivation_blocked?
  format.html { redirect_to admin_users_path, alert: t('users.owner_deactivation_rejected') }
  format.turbo_stream do
    flash.now[:alert] = t('users.owner_deactivation_rejected')
    render turbo_stream: turbo_stream.update('flash', Components::Layouts::Flash.new(alert: flash[:alert])),
           status: :unprocessable_content
  end
else
  @user.deactivate!
  format.html { redirect_to admin_users_path, notice: t('users.deactivated') }
  format.turbo_stream do
    flash.now[:notice] = t('users.deactivated')
    render turbo_stream: user_row_streams(@user)
  end
end
```

Add these private methods to `Admin::UsersController`:

```ruby
def owner_deactivation_blocked?
  !owner_governance.can_deactivate_owner_user?(existing_household_membership_for(@user))
end

def owner_governance
  @owner_governance ||= Admin::OwnerGovernance.new(
    household: admin_target_household,
    actor_membership: admin_target_membership
  )
end

def existing_household_membership_for(user)
  account = user.person&.account
  return unless admin_target_household && account

  admin_target_household.household_memberships.active.find_by(account: account)
end
```

- [ ] **Step 5: Add locale keys**

Modify `config/locales/en.yml` under `users`:

```yaml
    owner_deactivation_rejected: Owner accounts can only be deactivated by another household owner while another active owner user remains.
```

Modify `config/locales/cy.yml` under `users`:

```yaml
    owner_deactivation_rejected: Dim ond perchennog cartref arall all ddadactifadu cyfrif perchennog pan fydd defnyddiwr perchennog gweithredol arall yn parhau.
```

Modify `config/locales/es.yml` under `users`:

```yaml
    owner_deactivation_rejected: Las cuentas de propietario solo puede desactivarlas otro propietario del hogar mientras quede otro usuario propietario activo.
```

Modify `config/locales/ga.yml` under `users`:

```yaml
    owner_deactivation_rejected: Ni feidir cuntais uineara a dhighniomhachtu ach ag uineir teaghlaigh eile fad a fhanann usaideoir uineara gniomhach eile ann.
```

Modify `config/locales/pt.yml` under `users`:

```yaml
    owner_deactivation_rejected: As contas de proprietario so podem ser desativadas por outro proprietario do agregado enquanto existir outro utilizador proprietario ativo.
```

- [ ] **Step 6: Run the focused spec and verify it passes**

Run: `task test TEST_FILE=spec/requests/admin/user_mutation_boundary_spec.rb`

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add app/services/admin/owner_governance.rb app/controllers/admin/users_controller.rb config/locales/en.yml config/locales/cy.yml config/locales/es.yml config/locales/ga.yml config/locales/pt.yml spec/requests/admin/user_mutation_boundary_spec.rb
git commit -m "fix(security): protect owner user deactivation"
```

## Task 5: Route Profile Email Changes Through Rodauth Verification

**Files:**
- Modify: `app/controllers/profiles_controller.rb`
- Modify: `config/locales/en.yml`
- Modify: every supported locale file with matching key structure: `config/locales/cy.yml`, `config/locales/es.yml`, `config/locales/ga.yml`, `config/locales/pt.yml`
- Test: `spec/requests/profiles_show_spec.rb`

- [ ] **Step 1: Replace direct profile email update specs**

In `spec/requests/profiles_show_spec.rb`, replace the examples named `rejects a blank account email` and `updates the account email` with:

```ruby
it 'rejects direct profile email changes without mutating the account email' do
  original_email = account.email

  expect do
    patch profile_path, params: { account: { email: 'updated@example.test' } }
  end.not_to change { SecurityAuditEvent.where(event_type: 'auth_token/login_change_key/created').count }

  expect(response).to redirect_to('/change-login')
  expect(flash[:alert]).to include('Use the verified email change flow')
  expect(account.reload.email).to eq(original_email)
end

it 'creates a Rodauth login-change verification when using the change-login route' do
  original_email = account.email

  expect do
    post '/change-login', params: { email: 'verified-change@example.test', password: 'password' }
  end.to change { SecurityAuditEvent.where(event_type: 'auth_token/login_change_key/created').count }.by(1)

  expect(account.reload.email).to eq(original_email)
end
```

- [ ] **Step 2: Run the focused spec and verify it fails**

Run: `task test TEST_FILE=spec/requests/profiles_show_spec.rb`

Expected: FAIL because `ProfilesController#account_params` still permits `email` and updates `accounts.email` directly.

- [ ] **Step 3: Block direct email changes in the profile controller**

Modify `app/controllers/profiles_controller.rb`:

```ruby
def update
  @person = current_user.person
  @account = current_account
  authorize @person, :update?

  attributes = person_params
  return update_person_profile(attributes) if attributes.present?
  return respond_email_change_requires_verification if direct_email_change_requested?

  attributes = account_params
  return update_account_profile(attributes) if attributes.present?

  respond_no_changes
end
```

Change `account_params` so it no longer permits `email`:

```ruby
def account_params
  params.expect(account: %i[gravatar_enabled time_zone]) if params[:account]
end
```

Add these private methods:

```ruby
def direct_email_change_requested?
  params.dig(:account, :email).present?
end

def respond_email_change_requires_verification
  message = t('profiles.email_change_requires_verification')
  respond_to do |format|
    format.html { redirect_to '/change-login', alert: message }
    format.turbo_stream do
      flash.now[:alert] = message
      render turbo_stream: turbo_stream.update('flash', Components::Layouts::Flash.new(alert: flash[:alert])),
             status: :unprocessable_content
    end
  end
end
```

- [ ] **Step 4: Remove unreachable direct-email response methods**

Remove `respond_email_updated` and `respond_email_failed` from `app/controllers/profiles_controller.rb`, then simplify `update_account_profile`:

```ruby
def update_account_profile(attributes)
  if @account.update(attributes)
    respond_profile_updated(person: @person, account: @account)
  else
    respond_profile_failed(@account)
  end
end
```

- [ ] **Step 5: Add locale keys**

Modify `config/locales/en.yml` under `profiles`:

```yaml
  email_change_requires_verification: Use the verified email change flow to update your sign-in email.
```

Modify `config/locales/cy.yml` under `profiles`:

```yaml
  email_change_requires_verification: Defnyddiwch y llif newid e-bost wedi'i ddilysu i ddiweddaru eich e-bost mewngofnodi.
```

Modify `config/locales/es.yml` under `profiles`:

```yaml
  email_change_requires_verification: Usa el flujo verificado de cambio de correo para actualizar tu correo de inicio de sesion.
```

Modify `config/locales/ga.yml` under `profiles`:

```yaml
  email_change_requires_verification: Usaid an sreabhadh fioraithe athraithe riomhphoist chun do riomhphost sintithe isteach a nuashonru.
```

Modify `config/locales/pt.yml` under `profiles`:

```yaml
  email_change_requires_verification: Use o fluxo verificado de alteracao de email para atualizar o email de inicio de sessao.
```

- [ ] **Step 6: Run the focused spec and verify it passes**

Run: `task test TEST_FILE=spec/requests/profiles_show_spec.rb`

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add app/controllers/profiles_controller.rb config/locales/en.yml config/locales/cy.yml config/locales/es.yml config/locales/ga.yml config/locales/pt.yml spec/requests/profiles_show_spec.rb
git commit -m "fix(security): require verified login changes for profile email"
```

## Final Verification

- [ ] **Step 1: Run all focused security specs**

Run each command:

```bash
task test TEST_FILE=spec/services/portable_data/importer_spec.rb
task test TEST_FILE=spec/models/medication_spec.rb
task test TEST_FILE=spec/requests/api/v1/medications_spec.rb
task test TEST_FILE=spec/requests/admin/user_mutation_boundary_spec.rb
task test TEST_FILE=spec/requests/profiles_show_spec.rb
```

Expected: each command exits 0 with no failures.

- [ ] **Step 2: Run lint**

Run: `task rubocop`

Expected: exits 0 with no offenses.

- [ ] **Step 3: Run the full suite before opening a PR**

Run: `task test`

Expected: exits 0. If it does not, fix the failing specs before creating or updating the PR.

- [ ] **Step 4: Self-review the implementation**

Run:

```bash
git diff --check
git diff --stat
git diff -- app/services/portable_data/importer.rb app/models/medication.rb app/controllers/admin/users_controller.rb app/controllers/profiles_controller.rb
```

Expected: no whitespace errors, a focused diff, and no unrelated edits.

Check these points manually:

- Delegated imports cannot write location rows, medication rows, or dosage rows outside granted people.
- Owner and administrator portable imports remain full-fidelity household imports.
- `Medication` rejects mismatched `household` and `location` even when constructed outside controllers.
- Admin role changes and deactivation share the same owner-governance rules.
- Direct profile email updates cannot change `accounts.email`.
- Rodauth `/change-login` still creates a login-change verification event and leaves `accounts.email` unchanged until verification.

- [ ] **Step 5: Push**

```bash
git pull --rebase
git push
```

Expected: push succeeds and `git status --branch --short` shows the branch is up to date with origin.

## Self-Review

- Spec coverage: The plan maps finding 6 to Task 1, finding 7 to Task 2, finding 8 to Task 3, finding 9 to Task 4, and finding 10 to Task 5.
- Placeholder scan: The plan contains concrete file paths, code snippets, commands, and expected results for every task.
- Type consistency: The shared `Admin::OwnerGovernance` public methods are defined before both call sites use them; medication and profile helper names match the snippets that call them.

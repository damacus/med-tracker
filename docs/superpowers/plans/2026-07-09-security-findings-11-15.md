# Security Findings 11-15 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remediate MedTracker security findings 11-15: API login lockout bypass, missing platform step-up MFA, unsafe dm+d ZIP extraction, tenant-admin mutation of global dm+d barcode data, and unbounded health report ranges.

**Architecture:** Keep security decisions at mutation boundaries and share small service objects where the same rule spans controllers or tools. API password failures should update the same Rodauth-backed lockout tables that web login uses. Platform control-plane writes should require fresh privileged-action MFA in the platform base controller. ZIP extraction should move from raw `unzip` calls to a bounded, preflighted Ruby extractor. Report date parsing should use one range validator shared by HTML, PDF, and MCP entrypoints.

**Tech Stack:** Ruby 4.0.5, Rails 8.1.3, Rodauth lockout tables, Pundit policies, rubyzip, RSpec request/service/policy specs, Rails fixtures, `task` commands.

---

## File Structure

- Create `app/models/account_login_failure.rb` so API auth code can update Rodauth's `account_login_failures` table through Active Record.
- Create `app/services/api_login_failure_recorder.rb` to increment failed API login counters, create active `AccountLockout` rows, and clear failures after successful password login.
- Modify `app/controllers/api/v1/auth/sessions_controller.rb` to record API password failures before returning the generic invalid-credentials response.
- Modify `spec/requests/api/v1/auth/sessions_spec.rb` and create `spec/services/api_login_failure_recorder_spec.rb` for API lockout behavior.
- Modify `app/controllers/platform/base_controller.rb` so non-GET/HEAD platform actions require `require_privileged_action_mfa`.
- Modify `app/controllers/platform/support_access_sessions_controller.rb` to rely on the base platform MFA gate instead of a local duplicate.
- Modify `spec/requests/platform/settings_spec.rb`, `spec/requests/platform/users_spec.rb`, and `spec/requests/platform/support_access_sessions_spec.rb` for stale and fresh privileged-action MFA.
- Add `rubyzip` to `Gemfile` and `Gemfile.lock`.
- Modify `app/services/nhs_dmd/release_archive_extractor.rb` to validate and extract ZIP entries with containment, symlink, entry count, and expanded-size checks.
- Modify `app/services/nhs_dmd/release_import.rb` to use `NhsDmd::ReleaseArchiveExtractor` for nested `*GTIN.zip` extraction.
- Modify `spec/services/nhs_dmd/release_archive_extractor_spec.rb`, `spec/services/nhs_dmd/release_archive_import_spec.rb`, and `spec/services/nhs_dmd/release_import_spec.rb` for invalid archive and nested ZIP behavior.
- Modify `app/policies/admin_nhs_dmd_import_policy.rb` so global dm+d imports require active platform-admin access.
- Modify `spec/policies/admin_nhs_dmd_import_policy_spec.rb` and `spec/requests/admin/nhs_dmd_imports_spec.rb` for household-manager denial and platform-admin allowance.
- Create `app/services/reports/date_range.rb` to parse, default, and bound report date ranges.
- Modify `app/controllers/reports_controller.rb`, `app/controllers/health_history_reports_controller.rb`, and `app/mcp/med_tracker_mcp/tools/health_history_summary_tool.rb` to use `Reports::DateRange`.
- Modify `spec/services/reports/date_range_spec.rb`, `spec/requests/reports_spec.rb`, and `spec/mcp/med_tracker_mcp/tools_spec.rb` for the 180-day range cap.
- Modify `config/locales/en.yml`, `config/locales/cy.yml`, `config/locales/es.yml`, `config/locales/ga.yml`, and `config/locales/pt.yml` for the new report range messages.

## Task 1: Record API Password Failures Into Rodauth Lockout Tables

**Files:**
- Create: `app/models/account_login_failure.rb`
- Create: `app/services/api_login_failure_recorder.rb`
- Modify: `app/controllers/api/v1/auth/sessions_controller.rb`
- Test: `spec/requests/api/v1/auth/sessions_spec.rb`
- Test: `spec/services/api_login_failure_recorder_spec.rb`

- [ ] **Step 1: Write the failing API lockout request specs**

Add these examples inside `describe 'POST /api/v1/auth/login'` in `spec/requests/api/v1/auth/sessions_spec.rb`:

```ruby
it 'records API password failures and locks the account after five attempts' do
  create_api_household_for(user)

  expect do
    5.times do
      post api_v1_auth_login_path,
           params: { email: user.email_address, password: 'wrong-password' },
           as: :json
    end
  end.to change { AccountLockout.active.where(account_id: account.id).count }.from(0).to(1)

  expect(response).to have_http_status(:unauthorized)
  expect(response.parsed_body.dig('error', 'code')).to eq('invalid_credentials')
end

it 'clears accumulated API password failures after successful password login' do
  create_api_household_for(user)

  2.times do
    post api_v1_auth_login_path,
         params: { email: user.email_address, password: 'wrong-password' },
         as: :json
  end

  expect(AccountLoginFailure.find_by(account_id: account.id)&.number).to eq(2)

  post api_v1_auth_login_path,
       params: { email: user.email_address, password: 'password' },
       as: :json

  expect(response).to have_http_status(:created)
  expect(AccountLoginFailure.find_by(account_id: account.id)).to be_nil
end
```

- [ ] **Step 2: Write service specs for the failure recorder**

Create `spec/services/api_login_failure_recorder_spec.rb`:

```ruby
# frozen_string_literal: true

require 'rails_helper'

RSpec.describe ApiLoginFailureRecorder do
  fixtures :accounts

  let(:account) { accounts(:damacus) }

  before do
    AccountLoginFailure.where(account_id: account.id).delete_all
    AccountLockout.where(account_id: account.id).delete_all
  end

  it 'increments the login failure counter for a verified account' do
    2.times { described_class.record_failure(account) }

    expect(AccountLoginFailure.find_by!(account_id: account.id).number).to eq(2)
    expect(ApiAuthState.locked_out?(account)).to be(false)
  end

  it 'creates an active lockout on the fifth failure' do
    5.times { described_class.record_failure(account) }

    expect(ApiAuthState.locked_out?(account)).to be(true)
    expect(AccountLockout.find_by!(account_id: account.id).deadline).to be > Time.current
  end

  it 'does not record failures for blank accounts' do
    expect { described_class.record_failure(nil) }
      .not_to change(AccountLoginFailure, :count)
  end

  it 'clears existing failures after a successful login' do
    described_class.record_failure(account)

    expect { described_class.clear_failures(account) }
      .to change { AccountLoginFailure.where(account_id: account.id).count }.from(1).to(0)
  end
end
```

- [ ] **Step 3: Run the focused specs and verify they fail**

Run: `task test TEST_FILE=spec/requests/api/v1/auth/sessions_spec.rb`

Expected: FAIL because API password failures do not create `AccountLoginFailure` or `AccountLockout` rows.

Run: `task test TEST_FILE=spec/services/api_login_failure_recorder_spec.rb`

Expected: FAIL because `ApiLoginFailureRecorder` and `AccountLoginFailure` do not exist.

- [ ] **Step 4: Create the Active Record model for Rodauth failures**

Create `app/models/account_login_failure.rb`:

```ruby
# frozen_string_literal: true

class AccountLoginFailure < ApplicationRecord
  self.primary_key = :account_id

  belongs_to :account, inverse_of: false
end
```

- [ ] **Step 5: Create the API login failure recorder**

Create `app/services/api_login_failure_recorder.rb`:

```ruby
# frozen_string_literal: true

class ApiLoginFailureRecorder
  MAX_INVALID_LOGINS = 5
  LOCKOUT_DEADLINE_INTERVAL = 30.minutes

  class << self
    def record_failure(account)
      new(account).record_failure
    end

    def clear_failures(account)
      AccountLoginFailure.where(account_id: account&.id).delete_all if account
    end
  end

  def initialize(account)
    @account = account
  end

  def record_failure
    return if account.blank? || ApiAuthState.locked_out?(account)

    account.with_lock do
      failure = AccountLoginFailure.find_or_initialize_by(account: account)
      failure.number = failure.number.to_i + 1
      failure.save!
      lock_account! if failure.number >= MAX_INVALID_LOGINS
    end
  end

  private

  attr_reader :account

  def lock_account!
    lockout = AccountLockout.find_or_initialize_by(account: account)
    lockout.key = SecureRandom.urlsafe_base64(32)
    lockout.deadline = LOCKOUT_DEADLINE_INTERVAL.from_now
    lockout.save!
  end
end
```

- [ ] **Step 6: Wire the recorder into API login**

Modify `login_permitted?` in `app/controllers/api/v1/auth/sessions_controller.rb`:

```ruby
def login_permitted?(account, household_membership, password)
  return false unless account&.verified?
  return false if ApiAuthState.locked_out?(account)

  unless ApiAuthState.password_authenticated?(account, password)
    ApiLoginFailureRecorder.record_failure(account)
    return false
  end

  ApiLoginFailureRecorder.clear_failures(account)
  !ApiAuthState.mfa_configured?(account) &&
    account.person&.user&.active? &&
    household_membership.present?
end
```

- [ ] **Step 7: Run focused specs and verify they pass**

Run: `task test TEST_FILE=spec/services/api_login_failure_recorder_spec.rb`

Expected: PASS.

Run: `task test TEST_FILE=spec/requests/api/v1/auth/sessions_spec.rb`

Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add app/models/account_login_failure.rb app/services/api_login_failure_recorder.rb app/controllers/api/v1/auth/sessions_controller.rb spec/services/api_login_failure_recorder_spec.rb spec/requests/api/v1/auth/sessions_spec.rb
git commit -m "fix(security): record API login lockouts"
```

## Task 2: Require Fresh Privileged MFA For Platform Writes

**Files:**
- Modify: `app/controllers/platform/base_controller.rb`
- Modify: `app/controllers/platform/support_access_sessions_controller.rb`
- Test: `spec/requests/platform/settings_spec.rb`
- Test: `spec/requests/platform/users_spec.rb`
- Test: `spec/requests/platform/support_access_sessions_spec.rb`

- [ ] **Step 1: Add failing platform settings MFA specs**

Add this helper to `spec/requests/platform/settings_spec.rb`:

```ruby
def authenticate_platform_totp(account)
  secret = 'jbswy3dpehpk3pxp'
  visible_secret = RodauthApp.rodauth.allocate.send(:otp_hmac_secret, secret)
  AccountOtpKey.create!(id: account.id, key: secret, last_use: 5.minutes.ago)
  post '/otp-auth', params: { otp: ROTP::TOTP.new(visible_secret).at(Time.current) }
end
```

Add these examples inside `RSpec.describe 'Platform settings'`:

```ruby
it 'requires fresh privileged MFA before updating platform settings' do
  PlatformAdmin.create!(account: platform_user.person.account)
  sign_in(platform_user)

  patch platform_settings_path, params: { app_settings: { invite_only: '0' } }

  expect(response).to redirect_to(profile_path)
  expect(AppSettings.instance.reload.invite_only).not_to be(false)
end

it 'updates platform settings after fresh privileged MFA' do
  PlatformAdmin.create!(account: platform_user.person.account)
  sign_in(platform_user)
  authenticate_platform_totp(platform_user.person.account)

  patch platform_settings_path, params: { app_settings: { invite_only: '0' } }

  expect(response).to redirect_to(platform_settings_path)
  expect(AppSettings.instance.reload.invite_only).to be(false)
end
```

Update the existing platform settings write examples to call `authenticate_platform_totp(platform_user.person.account)` before `patch platform_settings_path`.

- [ ] **Step 2: Add failing platform users MFA specs**

Add this helper to `spec/requests/platform/users_spec.rb`:

```ruby
def authenticate_platform_totp(account)
  secret = 'jbswy3dpehpk3pxp'
  visible_secret = RodauthApp.rodauth.allocate.send(:otp_hmac_secret, secret)
  AccountOtpKey.create!(id: account.id, key: secret, last_use: 5.minutes.ago)
  post '/otp-auth', params: { otp: ROTP::TOTP.new(visible_secret).at(Time.current) }
end
```

Add this example:

```ruby
it 'requires fresh privileged MFA before changing system administrator access' do
  sign_in(platform_user)

  expect do
    patch platform_user_path(target_user), params: { platform_user: { system_administrator: '1' } }
  end.not_to change(PlatformAdmin.active, :count)

  expect(response).to redirect_to(profile_path)
  expect(target_user.person.account.platform_admin).to be_nil
end
```

Update existing platform user write examples to call `authenticate_platform_totp(platform_user.person.account)` before `patch platform_user_path(...)`.

- [ ] **Step 3: Run focused platform specs and verify they fail**

Run: `task test TEST_FILE=spec/requests/platform/settings_spec.rb`

Expected: FAIL because platform settings writes currently run without step-up MFA.

Run: `task test TEST_FILE=spec/requests/platform/users_spec.rb`

Expected: FAIL because platform user writes currently run without step-up MFA.

- [ ] **Step 4: Move the privileged-action MFA gate to platform write actions**

Modify `app/controllers/platform/base_controller.rb`:

```ruby
module Platform
  class BaseController < ApplicationController
    include HostedPrivilegedActionMfa

    before_action :require_platform_admin
    before_action :require_privileged_action_mfa, if: :platform_write_action?

    private

    def pundit_user
      AuthorizationContext.new(account: current_account, household: nil, membership: nil)
    end

    def require_platform_admin
      return if current_account&.platform_admin&.active?

      user_not_authorized
    end

    def platform_write_action?
      !request.get? && !request.head?
    end
  end
end
```

- [ ] **Step 5: Remove the duplicate support-access before action**

Remove this line from `app/controllers/platform/support_access_sessions_controller.rb`:

```ruby
before_action :require_privileged_action_mfa
```

- [ ] **Step 6: Run focused platform specs and verify they pass**

Run: `task test TEST_FILE=spec/requests/platform/settings_spec.rb`

Expected: PASS.

Run: `task test TEST_FILE=spec/requests/platform/users_spec.rb`

Expected: PASS.

Run: `task test TEST_FILE=spec/requests/platform/support_access_sessions_spec.rb`

Expected: PASS, including the existing stale-proof and support-session MFA examples.

- [ ] **Step 7: Commit**

```bash
git add app/controllers/platform/base_controller.rb app/controllers/platform/support_access_sessions_controller.rb spec/requests/platform/settings_spec.rb spec/requests/platform/users_spec.rb spec/requests/platform/support_access_sessions_spec.rb
git commit -m "fix(security): require platform write MFA"
```

## Task 3: Replace dm+d ZIP Shell Extraction With Safe Bounded Extraction

**Files:**
- Modify: `Gemfile`
- Modify: `Gemfile.lock`
- Modify: `app/services/nhs_dmd/release_archive_extractor.rb`
- Modify: `app/services/nhs_dmd/release_import.rb`
- Test: `spec/services/nhs_dmd/release_archive_extractor_spec.rb`
- Test: `spec/services/nhs_dmd/release_archive_import_spec.rb`
- Test: `spec/services/nhs_dmd/release_import_spec.rb`

- [ ] **Step 1: Add the ZIP library dependency**

Run: `task test:exec CMD='bundle add rubyzip'`

Expected: `Gemfile` and `Gemfile.lock` include `rubyzip`.

Run: `task test:build`

Expected: the test image rebuilds with the new dependency available.

- [ ] **Step 2: Write failing archive extractor specs**

Replace the shell-stubbing examples in `spec/services/nhs_dmd/release_archive_extractor_spec.rb` with real ZIP fixtures:

```ruby
# frozen_string_literal: true

require 'rails_helper'
require 'fileutils'
require 'tmpdir'
require 'zip'

RSpec.describe NhsDmd::ReleaseArchiveExtractor do
  let(:tmp_root) { Pathname.new(Dir.mktmpdir('release-extractor-spec', Rails.root.join('tmp'))) }
  let(:zip_path) { tmp_root.join('release.zip') }
  let(:destination) { tmp_root.join('extract') }

  after { FileUtils.rm_rf(tmp_root) }

  it 'extracts regular files inside the destination' do
    write_zip('f_ampp2_3000000.xml' => '<AMPP />', 'nested/f_gtin2_0000000.xml' => '<GTIN />')

    described_class.new.extract(zip_path, destination)

    expect(destination.join('f_ampp2_3000000.xml').read).to eq('<AMPP />')
    expect(destination.join('nested/f_gtin2_0000000.xml').read).to eq('<GTIN />')
  end

  it 'rejects traversal entries before writing files' do
    write_zip('../escape.txt' => 'owned', 'f_ampp2_3000000.xml' => '<AMPP />')

    expect { described_class.new.extract(zip_path, destination) }
      .to raise_error(described_class::Error, /unsafe ZIP entry/)

    expect(tmp_root.join('escape.txt')).not_to exist
    expect(destination).not_to exist
  end

  it 'rejects absolute entries before writing files' do
    write_zip('/tmp/absolute.txt' => 'owned')

    expect { described_class.new.extract(zip_path, destination) }
      .to raise_error(described_class::Error, /unsafe ZIP entry/)
  end

  it 'rejects oversized entries before writing files' do
    stub_const("#{described_class}::MAX_ENTRY_BYTES", 4)
    write_zip('f_ampp2_3000000.xml' => 'too-large')

    expect { described_class.new.extract(zip_path, destination) }
      .to raise_error(described_class::Error, /too large/)
  end

  it 'extracts only matching entries when a pattern is provided' do
    write_zip('f_ampp2_3000000.xml' => '<AMPP />', 'f_gtin2_0000000.xml' => '<GTIN />')

    described_class.new.extract(zip_path, destination, pattern: 'f_gtin2_0*.xml')

    expect(destination.join('f_gtin2_0000000.xml')).to exist
    expect(destination.join('f_ampp2_3000000.xml')).not_to exist
  end

  def write_zip(entries)
    Zip::File.open(zip_path.to_s, create: true) do |zip|
      entries.each do |name, content|
        zip.get_output_stream(name) { |io| io.write(content) }
      end
    end
  end
end
```

- [ ] **Step 3: Add a nested GTIN ZIP regression spec**

Add this example to `spec/services/nhs_dmd/release_import_spec.rb`:

```ruby
it 'extracts nested GTIN ZIPs through the safe archive extractor' do
  write_ampp_xml([{ appid: '111', nm: 'Nested GTIN Product' }])
  nested_zip = release_dir.join('release_GTIN.zip')
  Zip::File.open(nested_zip.to_s, create: true) do |zip|
    zip.get_output_stream('f_gtin2_0000000.xml') do |io|
      io.write('<GTIN_DETAILS><AMPPS><AMPP><AMPPID>111</AMPPID><GTINDATA><GTIN>5016298210989</GTIN><STARTDT>2020-01-01</STARTDT></GTINDATA></AMPP></AMPPS></GTIN_DETAILS>')
    end
  end

  result = importer.import(release_dir)

  expect(result.imported_count).to eq(1)
  expect(barcode_record('5016298210989')).to have_attributes(display: 'Nested GTIN Product')
end
```

- [ ] **Step 4: Run focused specs and verify they fail**

Run: `task test TEST_FILE=spec/services/nhs_dmd/release_archive_extractor_spec.rb`

Expected: FAIL because the extractor still shells out to `unzip` and accepts unsafe entries.

Run: `task test TEST_FILE=spec/services/nhs_dmd/release_import_spec.rb`

Expected: FAIL until nested extraction uses the safe extractor and `Zip` is required in the spec.

- [ ] **Step 5: Implement safe extraction**

Replace `app/services/nhs_dmd/release_archive_extractor.rb` with:

```ruby
# frozen_string_literal: true

require 'fileutils'
require 'pathname'
require 'zip'

module NhsDmd
  class ReleaseArchiveExtractor
    class Error < StandardError; end

    MAX_ENTRIES = 200
    MAX_ENTRY_BYTES = 100.megabytes
    MAX_TOTAL_BYTES = 500.megabytes

    def extract(zip_path, destination, pattern: nil)
      destination = Pathname.new(destination).expand_path
      entries = validated_entries(zip_path, destination, pattern)
      FileUtils.mkdir_p(destination)
      Zip::File.open(zip_path.to_s) do |zip_file|
        entries.each do |entry_name|
          extract_entry(zip_file.find_entry(entry_name), destination)
        end
      end
    rescue Zip::Error => e
      raise Error, "ZIP extraction failed: #{e.message}"
    end

    private

    def validated_entries(zip_path, destination, pattern)
      total_size = 0
      entries = []
      Zip::File.open(zip_path.to_s) do |zip_file|
        zip_file.each do |entry|
          next if pattern && !File.fnmatch?(pattern, entry.name, File::FNM_PATHNAME)

          validate_entry!(entry, destination)
          total_size += entry.size.to_i
          raise Error, 'ZIP extraction would exceed expanded size limit.' if total_size > MAX_TOTAL_BYTES

          entries << entry.name
          raise Error, 'ZIP extraction contains too many entries.' if entries.size > MAX_ENTRIES
        end
      end
      entries
    end

    def validate_entry!(entry, destination)
      raise Error, "unsafe ZIP entry: #{entry.name}" if unsafe_name?(entry.name)
      raise Error, "unsafe ZIP entry: #{entry.name}" if symlink_entry?(entry)
      raise Error, "ZIP entry is too large: #{entry.name}" if entry.size.to_i > MAX_ENTRY_BYTES

      target = destination.join(entry.name).cleanpath.expand_path
      return if target.to_s.start_with?("#{destination}/") || target == destination

      raise Error, "unsafe ZIP entry: #{entry.name}"
    end

    def unsafe_name?(name)
      path = Pathname.new(name)
      path.absolute? || path.each_filename.any? { |part| part == '..' }
    end

    def symlink_entry?(entry)
      return entry.symlink? if entry.respond_to?(:symlink?)

      entry.respond_to?(:ftype) && entry.ftype == :symlink
    end

    def extract_entry(entry, destination)
      target = destination.join(entry.name).cleanpath.expand_path
      if entry.directory?
        FileUtils.mkdir_p(target)
      else
        FileUtils.mkdir_p(target.dirname)
        entry.extract(target.to_s) { true }
      end
    end
  end
end
```

- [ ] **Step 6: Use the extractor for nested GTIN ZIPs**

Modify `app/services/nhs_dmd/release_import.rb`:

```ruby
def initialize(extractor: ReleaseArchiveExtractor.new)
  @extractor = extractor
end
```

Add the reader:

```ruby
attr_reader :extractor
```

Replace `extract_gtin_xml` with:

```ruby
def extract_gtin_xml(zip_path, dest)
  extractor.extract(zip_path, dest, pattern: 'f_gtin2_0*.xml')
end
```

- [ ] **Step 7: Run focused specs and verify they pass**

Run: `task test TEST_FILE=spec/services/nhs_dmd/release_archive_extractor_spec.rb`

Expected: PASS.

Run: `task test TEST_FILE=spec/services/nhs_dmd/release_archive_import_spec.rb`

Expected: PASS.

Run: `task test TEST_FILE=spec/services/nhs_dmd/release_import_spec.rb`

Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add Gemfile Gemfile.lock app/services/nhs_dmd/release_archive_extractor.rb app/services/nhs_dmd/release_import.rb spec/services/nhs_dmd/release_archive_extractor_spec.rb spec/services/nhs_dmd/release_archive_import_spec.rb spec/services/nhs_dmd/release_import_spec.rb
git commit -m "fix(security): safely extract dm+d archives"
```

## Task 4: Restrict Global dm+d Import Writes To Platform Admins

**Files:**
- Modify: `app/policies/admin_nhs_dmd_import_policy.rb`
- Test: `spec/policies/admin_nhs_dmd_import_policy_spec.rb`
- Test: `spec/requests/admin/nhs_dmd_imports_spec.rb`

- [ ] **Step 1: Rewrite the policy expectations**

Replace the household-manager examples in `spec/policies/admin_nhs_dmd_import_policy_spec.rb` with:

```ruby
it 'denies household managers because dm+d imports mutate global catalog data' do
  household, account, owner = household_with_owner(email: 'dmd-owner@example.test', name: 'DMD Owner Family')
  administrator = create_membership(household, email: 'dmd-admin@example.test', role: :administrator)
  member = create_membership(household, email: 'dmd-member@example.test', role: :member)

  expect(
    owner: new_permitted?(account: account, household: household, membership: owner),
    administrator: new_permitted?(account: administrator.account, household: household, membership: administrator),
    member: new_permitted?(account: member.account, household: household, membership: member)
  ).to eq(owner: false, administrator: false, member: false)
end

it 'permits active platform admins to import global dm+d data' do
  account = Account.create!(email: 'dmd-platform@example.test', status: :verified)
  PlatformAdmin.create!(account: account)
  context = AuthorizationContext.new(account: account, household: nil, membership: nil)

  expect(described_class.new(context, :import).new?).to be(true)
  expect(described_class.new(context, :import).create?).to be(true)
end
```

- [ ] **Step 2: Add request specs for denied household managers and allowed platform admins**

In `spec/requests/admin/nhs_dmd_imports_spec.rb`, add:

```ruby
it 'denies household administrators from opening the global dm+d import form' do
  sign_in(admin)

  get new_admin_nhs_dmd_import_path

  expect(response).to redirect_to(root_path)
end

it 'allows active platform admins to create an import run' do
  PlatformAdmin.create!(account: admin.person.account)
  sign_in(admin)
  upload = uploaded_zip('nhsbsa_dmd_release.zip')

  allow(NhsDmdImportJob).to receive(:perform_later)

  post admin_nhs_dmd_import_path, params: { nhs_dmd_import: { release_zip: upload } }

  expect(response).to redirect_to(new_admin_nhs_dmd_import_path)
  expect(NhsDmdImportJob).to have_received(:perform_later).with(instance_of(NhsDmdImport))
end
```

Update existing request examples that should still reach the dm+d import form or create endpoint by creating an active `PlatformAdmin` for `admin.person.account` before signing in.

- [ ] **Step 3: Run focused specs and verify they fail**

Run: `task test TEST_FILE=spec/policies/admin_nhs_dmd_import_policy_spec.rb`

Expected: FAIL because household owners and administrators are still allowed.

Run: `task test TEST_FILE=spec/requests/admin/nhs_dmd_imports_spec.rb`

Expected: FAIL because the controller still permits household managers and the old tests do not yet have platform-admin setup.

- [ ] **Step 4: Change the policy**

Modify `app/policies/admin_nhs_dmd_import_policy.rb`:

```ruby
# frozen_string_literal: true

class AdminNhsDmdImportPolicy < ApplicationPolicy
  def new?
    platform_admin?
  end

  def create?
    platform_admin?
  end
end
```

- [ ] **Step 5: Run focused specs and verify they pass**

Run: `task test TEST_FILE=spec/policies/admin_nhs_dmd_import_policy_spec.rb`

Expected: PASS.

Run: `task test TEST_FILE=spec/requests/admin/nhs_dmd_imports_spec.rb`

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add app/policies/admin_nhs_dmd_import_policy.rb spec/policies/admin_nhs_dmd_import_policy_spec.rb spec/requests/admin/nhs_dmd_imports_spec.rb
git commit -m "fix(security): restrict global dmd imports"
```

## Task 5: Bound Health Report Date Ranges

**Files:**
- Create: `app/services/reports/date_range.rb`
- Modify: `app/controllers/reports_controller.rb`
- Modify: `app/controllers/health_history_reports_controller.rb`
- Modify: `app/mcp/med_tracker_mcp/tools/health_history_summary_tool.rb`
- Modify: `config/locales/en.yml`
- Modify: `config/locales/cy.yml`
- Modify: `config/locales/es.yml`
- Modify: `config/locales/ga.yml`
- Modify: `config/locales/pt.yml`
- Test: `spec/services/reports/date_range_spec.rb`
- Test: `spec/requests/reports_spec.rb`
- Test: `spec/mcp/med_tracker_mcp/tools_spec.rb`

- [ ] **Step 1: Write the date range service spec**

Create `spec/services/reports/date_range_spec.rb`:

```ruby
# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Reports::DateRange do
  it 'defaults to the previous seven days ending today' do
    travel_to Time.zone.local(2026, 7, 9, 12) do
      range = described_class.parse(start_date: nil, end_date: nil)

      expect(range.start_date).to eq(Date.new(2026, 7, 3))
      expect(range.end_date).to eq(Date.new(2026, 7, 9))
    end
  end

  it 'parses supplied dates' do
    range = described_class.parse(start_date: '2026-01-01', end_date: '2026-01-31')

    expect(range.start_date).to eq(Date.new(2026, 1, 1))
    expect(range.end_date).to eq(Date.new(2026, 1, 31))
  end

  it 'rejects ranges longer than 180 days' do
    expect do
      described_class.parse(start_date: '2026-01-01', end_date: '2026-07-01')
    end.to raise_error(described_class::RangeTooLarge)
  end

  it 'rejects end dates before start dates' do
    expect do
      described_class.parse(start_date: '2026-02-01', end_date: '2026-01-01')
    end.to raise_error(ArgumentError)
  end
end
```

- [ ] **Step 2: Add report request specs**

Add these examples to `spec/requests/reports_spec.rb`:

```ruby
it 'redirects with alert when the report date range exceeds 180 days' do
  get reports_path, params: { start_date: '2026-01-01', end_date: '2026-07-01' }

  expect(response).to redirect_to(reports_path)
  expect(flash[:alert]).to eq('Report date range cannot exceed 180 days.')
end
```

Add this example inside `describe 'GET /reports/health-history'`:

```ruby
it 'redirects with alert when the PDF date range exceeds 180 days' do
  get health_history_report_path, params: { start_date: '2026-01-01', end_date: '2026-07-01' }

  expect(response).to redirect_to(reports_path)
  expect(flash[:alert]).to eq('Report date range cannot exceed 180 days.')
end
```

- [ ] **Step 3: Update MCP tests for the shared range validator**

In `spec/mcp/med_tracker_mcp/tools_spec.rb`, keep the existing health-history range test expectations but make sure the over-range case still returns:

```ruby
expect(response[:error]).to eq('Health history date range cannot exceed 180 days.')
```

- [ ] **Step 4: Run focused specs and verify they fail**

Run: `task test TEST_FILE=spec/services/reports/date_range_spec.rb`

Expected: FAIL because `Reports::DateRange` does not exist.

Run: `task test TEST_FILE=spec/requests/reports_spec.rb`

Expected: FAIL because the controllers accept over-wide ranges.

- [ ] **Step 5: Create the shared date range service**

Create `app/services/reports/date_range.rb`:

```ruby
# frozen_string_literal: true

module Reports
  class DateRange
    RangeTooLarge = Class.new(ArgumentError)

    MAX_RANGE_DAYS = 180
    DEFAULT_RANGE_DAYS = 6

    attr_reader :start_date, :end_date

    def self.parse(start_date:, end_date:, default_end_date: Time.zone.today, default_range_days: DEFAULT_RANGE_DAYS)
      end_on = end_date.present? ? Date.parse(end_date.to_s) : default_end_date
      start_on = start_date.present? ? Date.parse(start_date.to_s) : end_on - default_range_days
      new(start_date: start_on, end_date: end_on).tap(&:validate!)
    end

    def initialize(start_date:, end_date:)
      @start_date = start_date
      @end_date = end_date
    end

    def validate!
      raise ArgumentError, 'end_date must be on or after start_date' if end_date < start_date
      raise RangeTooLarge if (end_date - start_date).to_i > MAX_RANGE_DAYS

      self
    end

    def to_h
      { start_date: start_date, end_date: end_date }
    end
  end
end
```

- [ ] **Step 6: Use the service in the HTML reports controller**

Modify `app/controllers/reports_controller.rb`:

```ruby
def index
  authorize :report, :index?

  @date_range = Reports::DateRange.parse(start_date: params[:start_date], end_date: params[:end_date])
  @start_date = @date_range.start_date
  @end_date = @date_range.end_date

  @people = policy_scope(Person).order(:name, :id)
  @selected_person_id = params[:person_id].presence
  @filtered_people = filtered_people
  report_data = Reports::IndexQuery.new(people: @filtered_people, start_date: @start_date, end_date: @end_date).call
  today_taken_medications = Reports::TodayTakenMedicationsQuery.new(people: @filtered_people).call
  smart_insights = SmartInsights::IndexQuery.new(people: @filtered_people, start_date: @start_date, end_date: @end_date).call
  @daily_data = report_data.daily_data
  @inventory_alerts = report_data.inventory_alerts

  render Views::Reports::Index.new(
    daily_data: @daily_data,
    smart_insights: smart_insights,
    start_date: @start_date,
    end_date: @end_date,
    today_taken_medications: today_taken_medications,
    people: @people,
    selected_person_id: @selected_person_id
  )
rescue Reports::DateRange::RangeTooLarge
  redirect_to reports_path, alert: t('reports.date_range_too_large')
rescue ArgumentError
  redirect_to reports_path, alert: t('reports.invalid_date')
end
```

- [ ] **Step 7: Use the service in the PDF reports controller**

Modify `app/controllers/health_history_reports_controller.rb`:

```ruby
def show
  authorize :report, :index?

  send_data pdf_body,
            filename: filename,
            type: 'application/pdf',
            disposition: 'attachment'
rescue Reports::DateRange::RangeTooLarge
  redirect_to reports_path, alert: t('reports.date_range_too_large')
rescue ArgumentError
  redirect_to reports_path, alert: t('reports.invalid_date')
end
```

Replace `start_date` and `end_date` helpers with:

```ruby
def date_range
  @date_range ||= Reports::DateRange.parse(start_date: params[:start_date], end_date: params[:end_date])
end

def start_date
  date_range.start_date
end

def end_date
  date_range.end_date
end
```

- [ ] **Step 8: Reuse the service in the MCP health-history tool**

Modify `app/mcp/med_tracker_mcp/tools/health_history_summary_tool.rb` so `call` relies on `Reports::DateRange` to raise over-range errors:

```ruby
def call(server_context:, start_date: nil, end_date: nil, person_ids: nil)
  dates = date_range(start_date, end_date)

  context = tool_context(server_context)
  context.with_current do
    people = visible_people(context, person_ids)
    result = Reports::HealthHistoryQuery.new(
      people: people,
      start_date: dates.fetch(:start_date),
      end_date: dates.fetch(:end_date)
    ).call

    response(payload(result, dates), 'Bounded MedTracker health-history summary.')
  end
rescue Reports::DateRange::RangeTooLarge
  error_response('Health history date range cannot exceed 180 days.')
rescue ArgumentError
  error_response('start_date and end_date must be valid ISO8601 dates.')
end
```

Replace `date_range` with:

```ruby
def date_range(start_date, end_date)
  Reports::DateRange.parse(
    start_date: start_date,
    end_date: end_date,
    default_range_days: 30
  ).to_h
end
```

- [ ] **Step 9: Add locale keys**

Add this under `reports` in `config/locales/en.yml`:

```yaml
  date_range_too_large: Report date range cannot exceed 180 days.
```

Add matching keys to the remaining locale files:

```yaml
# config/locales/cy.yml
  date_range_too_large: Ni chaiff ystod dyddiadau'r adroddiad fod yn fwy na 180 diwrnod.

# config/locales/es.yml
  date_range_too_large: El intervalo de fechas del informe no puede superar los 180 dias.

# config/locales/ga.yml
  date_range_too_large: Ni feidir le raon datai na tuarascola dul thar 180 la.

# config/locales/pt.yml
  date_range_too_large: O intervalo de datas do relatorio nao pode exceder 180 dias.
```

- [ ] **Step 10: Run focused specs and verify they pass**

Run: `task test TEST_FILE=spec/services/reports/date_range_spec.rb`

Expected: PASS.

Run: `task test TEST_FILE=spec/requests/reports_spec.rb`

Expected: PASS.

Run: `task test TEST_FILE=spec/mcp/med_tracker_mcp/tools_spec.rb`

Expected: PASS.

- [ ] **Step 11: Commit**

```bash
git add app/services/reports/date_range.rb app/controllers/reports_controller.rb app/controllers/health_history_reports_controller.rb app/mcp/med_tracker_mcp/tools/health_history_summary_tool.rb config/locales/en.yml config/locales/cy.yml config/locales/es.yml config/locales/ga.yml config/locales/pt.yml spec/services/reports/date_range_spec.rb spec/requests/reports_spec.rb spec/mcp/med_tracker_mcp/tools_spec.rb
git commit -m "fix(security): bound report date ranges"
```

## Final Verification

- [ ] **Step 1: Run all focused security specs**

Run each command:

```bash
task test TEST_FILE=spec/requests/api/v1/auth/sessions_spec.rb
task test TEST_FILE=spec/services/api_login_failure_recorder_spec.rb
task test TEST_FILE=spec/requests/platform/settings_spec.rb
task test TEST_FILE=spec/requests/platform/users_spec.rb
task test TEST_FILE=spec/requests/platform/support_access_sessions_spec.rb
task test TEST_FILE=spec/services/nhs_dmd/release_archive_extractor_spec.rb
task test TEST_FILE=spec/services/nhs_dmd/release_archive_import_spec.rb
task test TEST_FILE=spec/services/nhs_dmd/release_import_spec.rb
task test TEST_FILE=spec/policies/admin_nhs_dmd_import_policy_spec.rb
task test TEST_FILE=spec/requests/admin/nhs_dmd_imports_spec.rb
task test TEST_FILE=spec/services/reports/date_range_spec.rb
task test TEST_FILE=spec/requests/reports_spec.rb
task test TEST_FILE=spec/mcp/med_tracker_mcp/tools_spec.rb
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
git diff -- app/controllers/api/v1/auth/sessions_controller.rb app/controllers/platform/base_controller.rb app/services/nhs_dmd/release_archive_extractor.rb app/policies/admin_nhs_dmd_import_policy.rb app/services/reports/date_range.rb
```

Expected: no whitespace errors, a focused diff, and no unrelated edits.

Check these points manually:

- API password failures increment `account_login_failures` and create active `account_lockouts` after five bad attempts.
- API success clears stale password failure counters.
- Platform `GET` pages remain readable to active platform admins, while platform writes require fresh privileged-action MFA.
- dm+d archive extraction rejects absolute paths, traversal paths, symlinks, too many entries, and oversized expanded content before writing files.
- Nested GTIN ZIP extraction uses the same safe extractor as top-level releases.
- Household managers cannot enqueue global dm+d imports; active platform admins can.
- HTML reports, health-history PDFs, and MCP health-history summaries all enforce the 180-day range cap.

- [ ] **Step 5: Push**

```bash
git pull --rebase
git push
```

Expected: push succeeds and `git status --branch --short` shows the branch is up to date with origin.

## Self-Review

- Spec coverage: Task 1 maps to finding 11, Task 2 maps to finding 12, Task 3 maps to finding 13, Task 4 maps to finding 14, and Task 5 maps to finding 15.
- Placeholder scan: The plan contains concrete file paths, code snippets, commands, and expected results for every task.
- Type consistency: `AccountLoginFailure`, `ApiLoginFailureRecorder`, `Reports::DateRange`, and `NhsDmd::ReleaseArchiveExtractor#extract(..., pattern:)` are defined before any later task uses them.

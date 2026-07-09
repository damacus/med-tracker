# Security Findings 16-18 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remediate MedTracker security findings 16-18: arbitrary Web Push egress endpoints, AI medication suggestion throttle path mismatch, and production Host header/auth URL hardening.

**Architecture:** Keep outbound URL trust decisions close to the data model and delivery sink, so invalid push endpoints cannot be stored or delivered if already present. Keep Rack Attack route matching in named constants that mirror tenant-scoped routes. Fail closed in production for public app origin configuration, and keep non-production fallbacks explicit for local development and tests.

**Tech Stack:** Ruby 4.0.5, Rails 8.1.3, Rack::Attack, WebPush, RSpec request/model/service/config specs, Rails fixtures, `task` commands.

---

## File Structure

- Create `app/services/push_subscription_endpoint_policy.rb` to parse and allow only supported HTTPS Web Push service endpoints.
- Modify `app/models/push_subscription.rb` to reject unsafe endpoints before persistence.
- Modify `app/services/push_notification_service.rb` to skip any legacy unsafe stored endpoint before `WebPush.payload_send`.
- Modify `spec/models/push_subscription_spec.rb`, `spec/requests/push_subscriptions_spec.rb`, `spec/services/push_notification_service_spec.rb`, `spec/jobs/low_stock_notification_job_spec.rb`, and `spec/jobs/missed_dose_notification_job_spec.rb` for valid/invalid endpoint behavior.
- Modify `config/initializers/rack_attack.rb` to match `/households/:household_slug/ai-medication-suggestions`.
- Create `spec/requests/ai_medication_rate_limiting_spec.rb` to prove AI-specific IP and user throttles apply to the routed path.
- Modify `config/environments/production.rb` to require `APP_URL`, derive `config.hosts`, and keep health checks excluded from host authorization.
- Modify `app/misc/rodauth_main.rb` to use a fail-closed app URL helper for WebAuthn and OIDC logout URL material.
- Modify `config/initializers/oidc_security.rb` to validate the same fail-closed APP_URL-derived redirect fallback.
- Modify `spec/security/zitadel_oidc_spec.rb`, `spec/security/oidc_security_spec.rb`, and create `spec/config/med_tracker/application_host_authorization_spec.rb` for host/auth URL hardening.

## Task 1: Validate Stored Web Push Endpoints Before Storage And Delivery

**Files:**
- Create: `app/services/push_subscription_endpoint_policy.rb`
- Modify: `app/models/push_subscription.rb`
- Modify: `app/services/push_notification_service.rb`
- Test: `spec/models/push_subscription_spec.rb`
- Test: `spec/requests/push_subscriptions_spec.rb`
- Test: `spec/services/push_notification_service_spec.rb`
- Test: `spec/jobs/low_stock_notification_job_spec.rb`
- Test: `spec/jobs/missed_dose_notification_job_spec.rb`

- [ ] **Step 1: Add failing model validation coverage**

Add these examples inside `describe 'validations'` in `spec/models/push_subscription_spec.rb`:

```ruby
it 'allows known browser Web Push service endpoints' do
  expect(
    valid_subscription(endpoint: 'https://fcm.googleapis.com/fcm/send/registration-token')
  ).to be_valid
  expect(
    valid_subscription(endpoint: 'https://updates.push.services.mozilla.com/wpush/v2/registration-token')
  ).to be_valid
  expect(
    valid_subscription(endpoint: 'https://web.push.apple.com/registration-token')
  ).to be_valid
end

it 'rejects non-HTTPS and local network endpoints' do
  endpoints = [
    'http://fcm.googleapis.com/fcm/send/registration-token',
    'https://localhost/push',
    'https://127.0.0.1/push',
    'https://10.0.0.5/push',
    'https://169.254.169.254/latest/meta-data',
    'https://example.com/push'
  ]

  endpoints.each do |endpoint|
    subscription = valid_subscription(endpoint: endpoint)

    expect(subscription).not_to be_valid
    expect(subscription.errors[:endpoint]).to include('must be a supported HTTPS Web Push endpoint')
  end
end
```

Add this helper to the bottom of the same spec:

```ruby
def valid_subscription(endpoint:)
  described_class.new(
    account: accounts(:admin),
    endpoint: endpoint,
    p256dh: 'p256dh_key',
    auth: 'auth_key'
  )
end
```

Update the spec fixtures line to `fixtures :accounts` if it is not already present.

- [ ] **Step 2: Add failing request coverage for unsafe subscription endpoints**

In `spec/requests/push_subscriptions_spec.rb`, add this example inside `describe 'POST /push_subscription'`:

```ruby
it 'rejects unsupported push service endpoints without storing them' do
  expect do
    post push_subscription_path,
         params: {
           endpoint: 'https://127.0.0.1/push/subscriptions/internal',
           keys: {
             p256dh: 'public_key',
             auth: 'auth_secret'
           }
         },
         as: :json
  end.not_to change(PushSubscription, :count)

  expect(response).to have_http_status(:unprocessable_content)
  expect(response.parsed_body['errors']).to include('Endpoint must be a supported HTTPS Web Push endpoint')
end
```

Replace existing valid push endpoint strings in `spec/requests/push_subscriptions_spec.rb` with the allowed FCM form:

```ruby
valid_endpoint = 'https://fcm.googleapis.com/fcm/send/registration-token'
```

- [ ] **Step 3: Add failing delivery guard coverage for legacy unsafe rows**

Add this example to `spec/services/push_notification_service_spec.rb` inside `describe '.send_to_account'`:

```ruby
it 'skips unsafe legacy web push endpoints before delivery' do
  PushSubscription.new(
    account: account,
    endpoint: 'https://127.0.0.1/push/internal',
    p256dh: 'legacy_public_key',
    auth: 'legacy_auth_secret'
  ).save!(validate: false)
  allow(WebPush).to receive(:payload_send)
  allow(Rails.logger).to receive(:warn)

  described_class.send_to_account(account, title: 'Medication Reminder', body: 'Take aspirin')

  expect(WebPush).to have_received(:payload_send).twice
  expect(Rails.logger).to have_received(:warn).with(/Skipped unsafe web push endpoint/)
end
```

Replace existing valid endpoint strings in `spec/services/push_notification_service_spec.rb`, `spec/jobs/low_stock_notification_job_spec.rb`, and `spec/jobs/missed_dose_notification_job_spec.rb` with allowed FCM endpoints such as:

```ruby
'https://fcm.googleapis.com/fcm/send/first'
'https://fcm.googleapis.com/fcm/send/second'
```

- [ ] **Step 4: Run focused specs and verify they fail**

Run:

```fish
task test TEST_FILE=spec/models/push_subscription_spec.rb
task test TEST_FILE=spec/requests/push_subscriptions_spec.rb
task test TEST_FILE=spec/services/push_notification_service_spec.rb
```

Expected:
- Model/request specs fail because no endpoint allowlist validation exists.
- Service spec fails because unsafe legacy rows are still delivered to `WebPush.payload_send`.

- [ ] **Step 5: Create the endpoint policy service**

Create `app/services/push_subscription_endpoint_policy.rb`:

```ruby
# frozen_string_literal: true

require 'ipaddr'
require 'uri'

class PushSubscriptionEndpointPolicy
  ALLOWED_HOSTS = %w[
    fcm.googleapis.com
    updates.push.services.mozilla.com
    web.push.apple.com
  ].freeze

  ALLOWED_HOST_SUFFIXES = %w[
    .push.apple.com
    .notify.windows.com
  ].freeze

  class << self
    def allowed?(endpoint)
      uri = URI.parse(endpoint.to_s)
      return false unless uri.is_a?(URI::HTTPS)
      return false if uri.host.blank? || uri.userinfo.present?

      host = uri.host.downcase
      allowed_host?(host) && !private_address?(host)
    rescue URI::InvalidURIError
      false
    end

    private

    def allowed_host?(host)
      ALLOWED_HOSTS.include?(host) ||
        ALLOWED_HOST_SUFFIXES.any? { |suffix| host.end_with?(suffix) }
    end

    def private_address?(host)
      address = IPAddr.new(host)
      address.loopback? || address.private? || address.link_local?
    rescue IPAddr::InvalidAddressError
      false
    end
  end
end
```

- [ ] **Step 6: Wire model validation**

Modify `app/models/push_subscription.rb`:

```ruby
# frozen_string_literal: true

class PushSubscription < ApplicationRecord
  belongs_to :account

  validates :endpoint, presence: true, uniqueness: true
  validates :p256dh, presence: true
  validates :auth, presence: true
  validate :endpoint_must_be_supported_push_service

  def to_webpush_params
    { endpoint: endpoint, p256dh: p256dh, auth: auth }
  end

  private

  def endpoint_must_be_supported_push_service
    return if endpoint.blank? || PushSubscriptionEndpointPolicy.allowed?(endpoint)

    errors.add(:endpoint, 'must be a supported HTTPS Web Push endpoint')
  end
end
```

- [ ] **Step 7: Skip unsafe legacy rows in the delivery sink**

Modify `deliver` in `app/services/push_notification_service.rb`:

```ruby
def self.deliver(sub, payload, vapid)
  unless PushSubscriptionEndpointPolicy.allowed?(sub.endpoint)
    Rails.logger.warn("Skipped unsafe web push endpoint for subscription #{sub.id}")
    return
  end

  WebPush.payload_send(
    message: payload,
    endpoint: sub.endpoint,
    p256dh: sub.p256dh,
    auth: sub.auth,
    vapid: vapid
  )
rescue WebPush::ExpiredSubscription, WebPush::InvalidSubscription
  sub.destroy
rescue StandardError => e
  Rails.logger.error("Push notification delivery failed for subscription #{sub.id}: #{e.class}: #{e.message}")
end
```

- [ ] **Step 8: Run focused specs and verify they pass**

Run:

```fish
task test TEST_FILE=spec/models/push_subscription_spec.rb
task test TEST_FILE=spec/requests/push_subscriptions_spec.rb
task test TEST_FILE=spec/services/push_notification_service_spec.rb
task test TEST_FILE=spec/jobs/low_stock_notification_job_spec.rb
task test TEST_FILE=spec/jobs/missed_dose_notification_job_spec.rb
```

Expected: PASS.

- [ ] **Step 9: Commit**

```fish
git add app/services/push_subscription_endpoint_policy.rb app/models/push_subscription.rb app/services/push_notification_service.rb spec/models/push_subscription_spec.rb spec/requests/push_subscriptions_spec.rb spec/services/push_notification_service_spec.rb spec/jobs/low_stock_notification_job_spec.rb spec/jobs/missed_dose_notification_job_spec.rb
git commit -m "fix(security): validate web push endpoints"
```

## Task 2: Match AI Medication Suggestion Rack Attack Throttles To Tenant Routes

**Files:**
- Modify: `config/initializers/rack_attack.rb`
- Create: `spec/requests/ai_medication_rate_limiting_spec.rb`

- [ ] **Step 1: Add failing Rack Attack request specs**

Create `spec/requests/ai_medication_rate_limiting_spec.rb`:

```ruby
# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'AI medication suggestion rate limiting' do
  include ActiveSupport::Testing::TimeHelpers

  fixtures :accounts, :people, :users, :households, :locations, :location_memberships

  let(:admin) { users(:admin) }
  let(:household) { Household.find_by!(slug: default_request_household_slug) }
  let(:suggestion) { AiMedication::Suggestion.new(medication: { description: 'Draft' }) }
  let(:service) { instance_double(AiMedication::SuggestionService, call: suggestion) }

  around do |example|
    original_cache_store = Rack::Attack.cache.store
    original_enabled = Rack::Attack.enabled

    Rack::Attack.cache.store = ActiveSupport::Cache::MemoryStore.new
    Rack::Attack.enabled = true

    example.run
  ensure
    Rack::Attack.cache.store = original_cache_store
    Rack::Attack.enabled = original_enabled
  end

  before do
    freeze_time
    sign_in(admin)
    household.update!(subscription_plan: 'family_plus')
    allow(ENV).to receive(:fetch).with('MEDTRACKER_AI_MEDICATION_HELP_ENABLED', 'false').and_return('true')
    allow(AiMedication::SuggestionService).to receive(:new).and_return(service)
  end

  it 'throttles tenant-scoped AI suggestions by IP' do
    notifications = []
    subscriber = lambda do |_name, _started, _finished, _id, payload|
      notifications << payload
    end

    ActiveSupport::Notifications.subscribed(subscriber, 'rack_attack.throttled') do
      10.times { post ai_medication_suggestions_path, params: suggestion_params }
      expect(response).to have_http_status(:ok)

      post ai_medication_suggestions_path, params: suggestion_params
    end

    expect(response).to have_http_status(:too_many_requests)
    expect(response.headers['Retry-After'].to_i).to be_positive
    expect(notifications).to include(hash_including(throttle: 'ai_medication_suggestions/ip'))
  end

  it 'throttles tenant-scoped AI suggestions by signed-in account' do
    20.times do |index|
      post ai_medication_suggestions_path,
           params: suggestion_params,
           headers: { 'REMOTE_ADDR' => "203.0.113.#{index}" }
    end
    expect(response).to have_http_status(:ok)

    post ai_medication_suggestions_path,
         params: suggestion_params,
         headers: { 'REMOTE_ADDR' => '203.0.113.250' }

    expect(response).to have_http_status(:too_many_requests)
    expect(response.headers['Retry-After'].to_i).to be_positive
  end

  def suggestion_params
    { medication: { name: 'Calpol Six Plus' } }
  end
end
```

- [ ] **Step 2: Run the new spec and verify it fails**

Run:

```fish
task test TEST_FILE=spec/requests/ai_medication_rate_limiting_spec.rb
```

Expected: FAIL because Rack Attack currently checks `/ai-medication-suggestions`, while the route is `/households/:household_slug/ai-medication-suggestions`.

- [ ] **Step 3: Update the Rack Attack path predicate**

Modify `config/initializers/rack_attack.rb`:

```ruby
MEDICATION_LOOKUP_PATH = %r{\A/households/[^/]+/medication-finder/search(?:\.[a-z]+)?\z}
AI_MEDICATION_SUGGESTIONS_PATH = %r{\A/households/[^/]+/ai-medication-suggestions\z}
MCP_PATH = '/mcp'
```

Replace the two AI throttles with:

```ruby
throttle('ai_medication_suggestions/ip', limit: 10, period: 1.minute) do |req|
  req.ip if req.path.match?(AI_MEDICATION_SUGGESTIONS_PATH) && req.post?
end

throttle('ai_medication_suggestions/user', limit: 20, period: 1.hour) do |req|
  if req.path.match?(AI_MEDICATION_SUGGESTIONS_PATH) && req.post?
    session = req.env['rack.session']
    session && session['account_id']
  end
end
```

- [ ] **Step 4: Run focused specs and verify they pass**

Run:

```fish
task test TEST_FILE=spec/requests/ai_medication_rate_limiting_spec.rb
task test TEST_FILE=spec/requests/ai_medication_suggestions_spec.rb
```

Expected: PASS.

- [ ] **Step 5: Commit**

```fish
git add config/initializers/rack_attack.rb spec/requests/ai_medication_rate_limiting_spec.rb
git commit -m "fix(security): throttle tenant AI suggestions"
```

## Task 3: Fail Closed On Production Host And Auth URL Configuration

**Files:**
- Modify: `config/environments/production.rb`
- Modify: `app/misc/rodauth_main.rb`
- Modify: `config/initializers/oidc_security.rb`
- Create: `spec/config/med_tracker/application_host_authorization_spec.rb`
- Modify: `spec/security/zitadel_oidc_spec.rb`
- Modify: `spec/security/oidc_security_spec.rb`

- [ ] **Step 1: Add failing production host configuration spec**

Create `spec/config/med_tracker/application_host_authorization_spec.rb`:

```ruby
# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MedTracker::Application do
  let(:production_config) { Rails.root.join('config/environments/production.rb').read }

  it 'requires APP_URL and derives production host authorization from it' do
    expect(production_config).to include("app_url = URI.parse(ENV.fetch('APP_URL'))")
    expect(production_config).to include("ENV.fetch('RAILS_ALLOWED_HOSTS', '')")
    expect(production_config).to include('config.hosts = allowed_hosts')
  end

  it 'keeps only health endpoints excluded from host authorization' do
    expect(production_config).to include("request.path == '/up' || request.path == '/health'")
  end
end
```

- [ ] **Step 2: Add failing auth URL fallback specs**

In `spec/security/zitadel_oidc_spec.rb`, replace the existing APP_URL fallback example:

```ruby
it 'falls back to request.base_url when APP_URL is not set' do
  expect(rodauth_source).to include("ENV.fetch('APP_URL', request.base_url)")
end
```

with:

```ruby
it 'uses a fail-closed app URL helper for logout redirect URIs' do
  expect(rodauth_source).to include('def medtracker_app_url')
  expect(rodauth_source).to include("raise KeyError, 'APP_URL is required in production' if Rails.env.production?")
  expect(rodauth_source).not_to include("ENV.fetch('APP_URL', request.base_url)")
end
```

In `spec/security/oidc_security_spec.rb`, replace the APP_URL fallback expectation:

```ruby
expect(initializer_file).to include("ENV.fetch('APP_URL', 'http://localhost:3000').delete_suffix('/')")
```

with:

```ruby
expect(initializer_file).to include("app_url = ENV.fetch('APP_URL') do")
expect(initializer_file).to include("raise KeyError, 'APP_URL is required in production' if Rails.env.production?")
```

- [ ] **Step 3: Run focused specs and verify they fail**

Run:

```fish
task test TEST_FILE=spec/config/med_tracker/application_host_authorization_spec.rb
task test TEST_FILE=spec/security/zitadel_oidc_spec.rb
task test TEST_FILE=spec/security/oidc_security_spec.rb
```

Expected: FAIL because production host authorization is still commented out and auth URL fallbacks still trust request host/base URL.

- [ ] **Step 4: Configure production host authorization from APP_URL**

Modify `config/environments/production.rb` lines around the mailer URL and host authorization:

```ruby
app_url = URI.parse(ENV.fetch('APP_URL'))
allowed_hosts = [
  app_url.host,
  *ENV.fetch('RAILS_ALLOWED_HOSTS', '').split(',').map(&:strip).reject(&:blank?)
].uniq

config.action_mailer.default_url_options = {
  host: app_url.host,
  protocol: app_url.scheme
}
```

Replace the commented host authorization block with:

```ruby
config.hosts = allowed_hosts

config.host_authorization = {
  exclude: ->(request) { request.path == '/up' || request.path == '/health' }
}
```

- [ ] **Step 5: Add the fail-closed Rodauth app URL helper**

In `app/misc/rodauth_main.rb`, add this helper inside the `rodauth do` block before the WebAuthn configuration:

```ruby
def medtracker_app_url
  ENV.fetch('APP_URL') do
    raise KeyError, 'APP_URL is required in production' if Rails.env.production?

    request.base_url
  end.delete_suffix('/')
end
```

Replace WebAuthn and OIDC logout URL material with:

```ruby
webauthn_rp_name 'MedTracker'
webauthn_rp_id { URI.parse(medtracker_app_url).host }
webauthn_origin { medtracker_app_url }
```

and:

```ruby
app_url = medtracker_app_url
redirect "#{end_session_url}?" \
         "id_token_hint=#{CGI.escape(@oidc_id_token_for_logout)}&" \
         "post_logout_redirect_uri=#{CGI.escape(app_url)}"
```

- [ ] **Step 6: Update OIDC security initializer fallback validation**

Modify `config/initializers/oidc_security.rb`:

```ruby
explicit_redirect_uri = ENV.fetch('OIDC_REDIRECT_URI', nil).presence
app_url = ENV.fetch('APP_URL') do
  raise KeyError, 'APP_URL is required in production' if Rails.env.production?

  'http://localhost:3000'
end.delete_suffix('/')
effective_redirect_uri = explicit_redirect_uri || "#{app_url}/auth/oidc/callback"
```

- [ ] **Step 7: Run focused specs and verify they pass**

Run:

```fish
task test TEST_FILE=spec/config/med_tracker/application_host_authorization_spec.rb
task test TEST_FILE=spec/security/zitadel_oidc_spec.rb
task test TEST_FILE=spec/security/oidc_security_spec.rb
```

Expected: PASS.

- [ ] **Step 8: Commit**

```fish
git add config/environments/production.rb app/misc/rodauth_main.rb config/initializers/oidc_security.rb spec/config/med_tracker/application_host_authorization_spec.rb spec/security/zitadel_oidc_spec.rb spec/security/oidc_security_spec.rb
git commit -m "fix(security): require production app host"
```

## Final Verification

- [ ] **Step 1: Run all focused security specs**

Run:

```fish
task test TEST_FILE=spec/models/push_subscription_spec.rb
task test TEST_FILE=spec/requests/push_subscriptions_spec.rb
task test TEST_FILE=spec/services/push_notification_service_spec.rb
task test TEST_FILE=spec/jobs/low_stock_notification_job_spec.rb
task test TEST_FILE=spec/jobs/missed_dose_notification_job_spec.rb
task test TEST_FILE=spec/requests/ai_medication_rate_limiting_spec.rb
task test TEST_FILE=spec/requests/ai_medication_suggestions_spec.rb
task test TEST_FILE=spec/config/med_tracker/application_host_authorization_spec.rb
task test TEST_FILE=spec/security/zitadel_oidc_spec.rb
task test TEST_FILE=spec/security/oidc_security_spec.rb
```

Expected: each command exits 0 with no failures.

- [ ] **Step 2: Run lint**

Run:

```fish
task rubocop
```

Expected: exits 0 with no offenses.

- [ ] **Step 3: Run the full suite before opening a PR**

Run:

```fish
task test
```

Expected: exits 0. Fix any failures before creating the PR.

- [ ] **Step 4: Self-review the implementation**

Run:

```fish
git diff --check
git diff --stat origin/main...HEAD
git diff origin/main...HEAD -- app/models/push_subscription.rb app/services/push_notification_service.rb config/initializers/rack_attack.rb config/environments/production.rb app/misc/rodauth_main.rb config/initializers/oidc_security.rb
```

Manually verify:
- Push subscription creation rejects `http`, localhost, RFC1918, link-local, and unrecognized domains.
- Existing unsafe push subscriptions are skipped before `WebPush.payload_send`.
- AI medication suggestion throttles match the tenant-scoped route by IP and signed-in account.
- Production requires `APP_URL`, configures `config.hosts`, and excludes only `/up` and `/health` from host authorization.
- WebAuthn and OIDC logout URL generation no longer fall back to request host/base URL in production.

- [ ] **Step 5: Push**

Run:

```fish
git pull --rebase
git push
```

Expected: branch is pushed and `git status --short --branch` shows it is up to date with origin.

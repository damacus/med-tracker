# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'tenant safety architecture' do
  let(:cache_allowed_paths) do
    %w[
      app/services/api/oidc_provider_client.rb
      app/services/nhs_dmd/barcode_lookup.rb
      app/services/nhs_dmd/client.rb
      app/services/nhs_website_content/client.rb
      app/services/open_food_facts/client.rb
      app/services/open_products_facts/client.rb
    ]
  end

  let(:controller_tenant_find) do
    /
      \b(?:Person|Location|Medication|Schedule|PersonMedication|MedicationTake|NotificationPreference)
      \.find
      \(
      \s*params
    /x
  end

  it 'does not load tenant records directly from params in controllers' do
    offenders = Rails.root.glob('app/controllers/**/*.rb').filter_map do |path|
      matches = File.readlines(path).each_with_index.filter_map do |line, index|
        "#{path}:#{index + 1}" if line.match?(controller_tenant_find)
      end
      matches.presence
    end.flatten

    expect(offenders).to be_empty
  end

  it 'requires medication reminder jobs to carry household identity before person identity' do
    reminder_source = Rails.root.join('app/jobs/medication_reminder_job.rb').read
    scheduler_source = Rails.root.join('app/jobs/schedule_daily_reminders_job.rb').read

    expect(reminder_source).to include('def perform(household_id, person_id, period, scheduled_time = nil)')
    expect(scheduler_source).to include('perform_later(pref.household_id, pref.person_id')
    expect(scheduler_source).not_to include('perform_later(pref.person_id')
  end

  it 'requires offline IndexedDB access to be namespaced by tenant key' do
    offline_store = Rails.root.join('app/javascript/controllers/offline_store.js').read

    expect(offline_store).to include('tenantKey')
    expect(offline_store).to include('snapshotKey(tenantKey)')
    expect(offline_store).to include('take.household_key === tenantKey')
  end

  it 'keeps tenant Turbo stream targets household-partitioned' do
    expect(unsafe_turbo_target_offenders).to be_empty
  end

  it 'keeps API data resources behind household routes' do
    paths = Rails.application.routes.routes.map { |route| route.path.spec.to_s }
    unscoped_api_data_paths = paths.grep(unscoped_api_data_path_pattern)

    expect(unscoped_api_data_paths).to be_empty
    expect(paths).to include('/api/v1/households/:household_id/people(.:format)')
  end

  it 'keeps tenant web resources behind household routes' do
    paths = Rails.application.routes.routes.map { |route| route.path.spec.to_s }
    unscoped_web_paths = paths.grep(unscoped_web_data_path_pattern)

    expect(unscoped_web_paths).to be_empty
    expect(paths.grep(%r{\A/admin(?:/|\(|\z)})).to be_empty
    expect(paths).to include('/households/:household_slug/admin(.:format)')
    expect(paths).to include('/households/:household_slug/people(.:format)')
    expect(paths).to include('/households/:household_slug/dashboard(.:format)')
  end

  it 'keeps Rails.cache usage out of tenant-owned application code' do
    offenders = Rails.root.glob('app/**/*.rb').filter_map do |path|
      relative_path = path.relative_path_from(Rails.root).to_s
      next if cache_allowed_paths.include?(relative_path)

      matches = File.readlines(path).each_with_index.filter_map do |line, index|
        "#{relative_path}:#{index + 1}" if line.include?('Rails.cache')
      end
      matches.presence
    end.flatten

    expect(offenders).to be_empty
  end

  it 'keeps tenant people lookups out of view components' do
    offenders = Rails.root.glob('app/components/**/*.rb').filter_map do |path|
      matches = File.readlines(path).each_with_index.filter_map do |line, index|
        "#{path.relative_path_from(Rails.root)}:#{index + 1}" if line.match?(/\bPerson\.(?:all|find|find_by|where)\b/)
      end
      matches.presence
    end.flatten

    expect(offenders).to be_empty
  end

  it 'keeps default Active Storage routes disabled for tenant-owned attachments' do
    expect(Rails.application.config.active_storage.draw_routes).to be(false)
    expect(route_paths).not_to include('/rails/active_storage/direct_uploads(.:format)')
  end

  it 'serves avatars through an application-owned household route' do
    expect(route_paths).to include('/households/:household_slug/people/:person_id/avatar(.:format)')
    expect(Rails.root.join('app/components/shared/person_avatar.rb').read).not_to include('url_for(person.avatar)')
  end

  it 'requires onboarding services to authorize derived plan records before saving' do
    source = Rails.root.join('app/services/medication_onboarding_create_service.rb').read

    expect(source).to include('plan_authorizer:')
    expect(source).to include('authorize_plan_record!(record)')
    expect(source.index('authorize_plan_record!(record)')).to be < source.index('record.save')
    expect(Rails.root.join('app/controllers/medications_controller.rb').read).to include(
      'plan_authorizer: ->(record) { authorize(record, :create?) }'
    )
  end

  it 'keeps API password verification behind the shared auth state boundary' do
    source = Rails.root.join('app/controllers/api/v1/auth/sessions_controller.rb').read

    expect(source).not_to include('BCrypt::Password')
    expect(source).to include('ApiAuthState')
  end

  it 'requires API bearer authentication to reject locked accounts and support app tokens' do
    source = Rails.root.join('app/controllers/api/v1/base_controller.rb').read

    expect(source).to include('ApiAppToken.lookup_by_token')
    expect(source).to include('ApiAuthState.locked_out?')
  end

  it 'keeps unsafe Pundit verification skips out of mutating app controllers' do
    skipped_controllers = Rails.root.glob('app/controllers/**/*.rb').filter_map do |path|
      next unless path.read.include?('skip_after_action :verify_pundit_authorization')

      path.relative_path_from(Rails.root).to_s
    end

    expect(skipped_controllers).to match_array(
      %w[
        app/controllers/household_redirects_controller.rb
        app/controllers/invitations_controller.rb
        app/controllers/pwa_controller.rb
        app/controllers/rodauth_controller.rb
      ]
    )
  end

  it 'keeps Pundit policies off legacy User role predicates' do
    offenders = Rails.root.glob('app/policies/**/*.rb').filter_map do |path|
      relative_path = path.relative_path_from(Rails.root).to_s
      matches = File.readlines(path).each_with_index.filter_map do |line, index|
        "#{relative_path}:#{index + 1}" if line.match?(legacy_policy_role_pattern)
      end
      matches.presence
    end.flatten

    expect(offenders).to be_empty
  end

  it 'removes legacy authorization and subscription columns from the cutover schema' do
    expect(connection_column_names(:users)).not_to include('role')
    expect(connection_column_names(:accounts)).not_to include('subscription_plan')
  end

  def unsafe_turbo_target_offenders
    Rails.root.glob('app/controllers/**/*.rb').filter_map do |path|
      matches = File.readlines(path).each_with_index.filter_map do |line, index|
        "#{path.relative_path_from(Rails.root)}:#{index + 1}" if line.match?(unsafe_turbo_target_pattern)
      end
      matches.presence
    end.flatten
  end

  def connection_column_names(table_name)
    ActiveRecord::Base.connection.columns(table_name).map(&:name)
  end

  def route_paths
    Rails.application.routes.routes.map { |route| route.path.spec.to_s }
  end

  def unsafe_turbo_target_pattern
    /
      turbo_stream\.(?:replace|remove|prepend|append)
      \(["']
      (?:people|locations|medications|
      (?:person|medication|location|schedule|person_medication|timeline)(?:_show)?_\#\{)
    /x
  end

  def legacy_policy_role_pattern
    /
      legacy_user|legacy_|
      \buser\.(?:administrator|doctor|nurse|carer|parent)\?|
      \b(?:admin_or_clinician|doctor|nurse|medical_staff|carer_or_parent)\?
    /x
  end

  def unscoped_api_data_path_pattern
    %r{
      \A/api/v1/
      (?:me|people|locations|medications|schedules|person_medications|medication_takes|notification_preference)
      (?:/|\(|\z)
    }x
  end

  def unscoped_web_data_path_pattern
    %r{
      \A/
      (?:
        search|dashboard|add_medication|profile|reports|offline|locations|medications|
        medication-finder|ai-medication-suggestions|schedules|people|push_subscription|
        native_device_tokens|notification_preference
      )
      (?:/|\(|\z)
    }x
  end
end

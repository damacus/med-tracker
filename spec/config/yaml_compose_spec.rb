# frozen_string_literal: true

require 'rails_helper'

RSpec.describe YAML do
  let(:compose_config) do
    described_class.safe_load(Rails.root.join('compose.yaml').read, aliases: true)
  end
  let(:init_roles_sql) { Rails.root.join('compose/init-roles.sql').read }

  it 'isolates public assets in development web container' do
    expect(compose_config.dig('services', 'web-dev', 'tmpfs')).to include('/app/public/assets:uid=1000,gid=1000')
  end

  it 'isolates public assets in test web container' do
    expect(compose_config.dig('services', 'web-test', 'tmpfs')).to include('/app/public/assets:uid=1000,gid=1000')
  end

  it 'mounts development PostgreSQL data at the PostgreSQL 18 data root' do
    expect(compose_config.dig('services', 'db-dev', 'volumes')).to include(
      'medtracker_dev_postgres:/var/lib/postgresql'
    )
  end

  it 'mounts production PostgreSQL data at the PostgreSQL 18 data root' do
    expect(compose_config.dig('services', 'db-prod', 'volumes')).to include(
      'medtracker_prod_postgres:/var/lib/postgresql'
    )
  end

  it 'bootstraps database runtime roles before Rails containers connect' do
    %w[db-dev db-test db-prod].each do |service_name|
      expect(compose_config.dig('services', service_name, 'volumes')).to include(
        './compose/init-roles.sql:/docker-entrypoint-initdb.d/001-init-roles.sql:ro'
      )
    end
  end

  it 'builds the development migrate container from the development image target' do
    expect(compose_config.dig('services', 'migrate-dev', 'image')).to eq('med-tracker-web-dev')
    expect(compose_config.dig('services', 'migrate-dev', 'build', 'target')).to eq('development')
    expect(compose_config.dig('services', 'migrate-dev', 'build', 'args', 'RAILS_ENV')).to eq('development')
  end

  it 'builds the test migrate container from the test image target' do
    expect(compose_config.dig('services', 'migrate-test', 'image')).to eq('med-tracker-web-test')
    expect(compose_config.dig('services', 'migrate-test', 'build', 'target')).to eq('test')
  end

  it 'passes OIDC environment through to Rails containers used for local OIDC flows' do
    expected_keys = %w[
      APP_URL
      OIDC_CLIENT_ID
      OIDC_CLIENT_SECRET
      OIDC_ISSUER_URL
      OIDC_PROVIDER_NAME
      OIDC_REDIRECT_URI
    ]

    expected_keys.each do |key|
      expect(compose_config.dig('services', 'migrate-dev', 'environment')).to include(key)
      expect(compose_config.dig('services', 'web-dev', 'environment')).to include(key)
      expect(compose_config.dig('services', 'migrate-test', 'environment')).to include(key)
      expect(compose_config.dig('services', 'web-test', 'environment')).to include(key)
    end
  end

  it 'uses separate database roles for migration and runtime containers' do
    services = %w[migrate-dev web-dev migrate-test web-test migrate-prod web-prod]
    database_roles = services.index_with do |service|
      compose_config.dig('services', service, 'environment', 'DATABASE_ROLE')
    end

    expect(database_roles).to eq(
      'migrate-dev' => '${DEV_MIGRATION_DATABASE_ROLE:-}',
      'web-dev' => '${DEV_DATABASE_ROLE:-}',
      'migrate-test' => '${TEST_MIGRATION_DATABASE_ROLE:-}',
      'web-test' => '${TEST_DATABASE_ROLE:-}',
      'migrate-prod' => '${MIGRATION_DATABASE_ROLE:-med_tracker_owner}',
      'web-prod' => '${DATABASE_ROLE:-med_tracker_app}'
    )
  end

  it 'keeps the production web container behind the runtime role after migrations complete' do
    expect(compose_config.dig('services', 'web-prod', 'depends_on', 'migrate-prod', 'condition'))
      .to eq('service_completed_successfully')
    expect(compose_config.dig('services', 'migrate-prod', 'environment', 'DATABASE_ROLE'))
      .to eq('${MIGRATION_DATABASE_ROLE:-med_tracker_owner}')
    expect(compose_config.dig('services', 'web-prod', 'environment', 'DATABASE_ROLE'))
      .to eq('${DATABASE_ROLE:-med_tracker_app}')
  end

  it 'bootstraps the deployed runtime role without owner or bypassrls privileges' do
    expect(init_roles_sql).to include('ALTER ROLE med_tracker_app NOLOGIN NOSUPERUSER NOBYPASSRLS;')
    expect(init_roles_sql).to include('GRANT USAGE ON SCHEMA public TO med_tracker_app;')
    expect(init_roles_sql).not_to match(/GRANT\s+USAGE,\s+CREATE\s+ON\s+SCHEMA\s+public\s+TO\s+med_tracker_app/i)
  end

  it 'keeps normal test runs isolated from local development OIDC credentials' do
    test_environment = compose_config.dig('services', 'migrate-test', 'environment')

    expect(test_environment.slice('APP_URL', 'OIDC_CLIENT_ID', 'OIDC_CLIENT_SECRET', 'OIDC_ISSUER_URL',
                                  'OIDC_PROVIDER_NAME', 'OIDC_REDIRECT_URI')).to eq(
                                    'APP_URL' => '${TEST_APP_URL:-http://localhost:3000}',
                                    'OIDC_CLIENT_ID' => '${TEST_OIDC_CLIENT_ID:-}',
                                    'OIDC_CLIENT_SECRET' => '${TEST_OIDC_CLIENT_SECRET:-}',
                                    'OIDC_ISSUER_URL' => '${TEST_OIDC_ISSUER_URL:-}',
                                    'OIDC_PROVIDER_NAME' => '${TEST_OIDC_PROVIDER_NAME:-OIDC}',
                                    'OIDC_REDIRECT_URI' => '${TEST_OIDC_REDIRECT_URI:-}'
                                  )
  end

  it 'keeps development and test web host ports Docker-assigned for parallel worktrees' do
    expect(compose_config.dig('services', 'web-dev', 'ports')).to be_nil
    expect(compose_config.dig('services', 'web-test', 'ports')).to be_nil
  end
end

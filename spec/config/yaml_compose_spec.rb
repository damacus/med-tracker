# frozen_string_literal: true

require 'rails_helper'

RSpec.describe YAML do
  let(:compose_config) do
    described_class.safe_load(Rails.root.join('compose.yaml').read, aliases: true)
  end
  let(:base_compose_config) do
    described_class.safe_load(Rails.root.join('compose/base.yml').read, aliases: true)
  end
  let(:init_roles_sql) { Rails.root.join('compose/init-roles.sql').read }
  let(:dockerfile) { Rails.root.join('Dockerfile').read }
  let(:deploy_config) { Rails.root.join('config/deploy.yml').read }

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

  it 'confines bootstrap fixture access to the ephemeral test runner', :aggregate_failures do
    runner = compose_config.dig('services', 'test-runner')

    expect(runner.dig('environment', 'DATABASE_URL'))
      .to eq('postgresql://medtracker:medtracker_password@db-test:5432/medtracker')
    expect(runner.dig('environment', 'DATABASE_ROLE')).to be_nil

    %w[migrate-dev web-dev migrate-test web-test migrate-prod web-prod].each do |service_name|
      expect(compose_config.dig('services', service_name, 'environment').to_json)
        .not_to include('medtracker_password')
    end
  end

  it 'prepares non-production databases and migrates production through the owner login' do
    expect(compose_config.dig('services', 'migrate-dev', 'command')).to eq('bin/rails db:prepare')
    expect(compose_config.dig('services', 'migrate-test', 'command')).to eq('bin/rails db:prepare')
    expect(compose_config.dig('services', 'migrate-prod', 'command')).to eq('bin/rails db:migrate')
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
      'migrate-dev' => '${DEV_MIGRATION_DATABASE_ROLE:-med_tracker_owner}',
      'web-dev' => '${DEV_DATABASE_ROLE:-med_tracker_app}',
      'migrate-test' => '${TEST_MIGRATION_DATABASE_ROLE:-med_tracker_owner}',
      'web-test' => '${TEST_DATABASE_ROLE:-med_tracker_app}',
      'migrate-prod' => '${MIGRATION_DATABASE_ROLE:-med_tracker_owner}',
      'web-prod' => '${DATABASE_ROLE:-med_tracker_app}'
    )
  end

  it 'uses distinct non-superuser logins at every Rails database boundary', :aggregate_failures do
    expected_database_urls.each do |service_name, urls|
      expect(compose_config.dig('services', service_name, 'environment')).to include(urls)
      expect(compose_config.dig('services', service_name, 'environment').to_json)
        .not_to include('medtracker_password')
    end
    expect(base_compose_config.dig('services', 'postgres', 'environment', 'POSTGRES_PASSWORD'))
      .to eq('medtracker_password')
  end

  it 'grants each primary login only its intended SET ROLE membership', :aggregate_failures do
    expect_primary_login_memberships
  end

  it 'repairs unsafe group roles and public schema creation', :aggregate_failures do
    expect(init_roles_sql).to include(
      'ALTER ROLE med_tracker_owner NOLOGIN NOSUPERUSER NOCREATEROLE NOCREATEDB NOREPLICATION NOBYPASSRLS;',
      'ALTER ROLE med_tracker_app NOLOGIN NOSUPERUSER NOCREATEROLE NOCREATEDB NOREPLICATION NOBYPASSRLS;',
      'REVOKE CREATE ON SCHEMA public FROM PUBLIC;'
    )
  end

  it 'loads deployment passwords from the environment and fails fast', :aggregate_failures do
    expect(init_roles_sql).to start_with("\\set ON_ERROR_STOP on\n")
    expect(init_roles_sql).to include(
      '\\getenv runtime_password RUNTIME_DATABASE_PASSWORD',
      '\\getenv migration_password MIGRATION_DATABASE_PASSWORD',
      '\\getenv auxiliary_password AUXILIARY_DATABASE_PASSWORD'
    )
  end

  it 'isolates the auxiliary login from the primary database', :aggregate_failures do
    expect(init_roles_sql).to include(
      '\\if :{?auxiliary_password}',
      "PASSWORD %L NOSUPERUSER NOCREATEROLE NOCREATEDB NOREPLICATION NOBYPASSRLS', :'auxiliary_password'",
      "format('REVOKE CONNECT ON DATABASE %I FROM PUBLIC', current_database())"
    )
    expect(init_roles_sql).not_to match(/GRANT\s+CONNECT\s+ON\s+DATABASE.*TO[^;]*medtracker_auxiliary/i)
    expect(init_multiple_databases).to include(
      'CREATE DATABASE $db OWNER medtracker_auxiliary',
      'ALTER DATABASE $db OWNER TO medtracker_auxiliary',
      'ALTER SCHEMA public OWNER TO medtracker_auxiliary',
      'REVOKE CREATE ON SCHEMA public FROM PUBLIC'
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

  it 'bakes the exact tagged production image reference without exposing a runtime override', :aggregate_failures do
    expected_image = '${APP_IMAGE_REF:-med-tracker:local-production-build}'

    %w[migrate-prod web-prod audit-exporter-prod audit-verifier-prod].each do |service_name|
      service = compose_config.dig('services', service_name)
      expect(service.fetch('image')).to eq(expected_image)
      expect(service.dig('build', 'args', 'APP_IMAGE_REF')).to eq(expected_image)
      expect(service.fetch('environment')).not_to include('RUNTIME_APP_IMAGE')
    end
    expect(dockerfile).to include('ARG APP_IMAGE_REF', '/app/.runtime-image-ref')
    expect(dockerfile).not_to include('ENV RUNTIME_APP_IMAGE')
    expect(deploy_config).to include('APP_IMAGE_REF: <%= ENV.fetch("APP_IMAGE_REF") %>')
  end

  it 'runs the audit exporter with isolated database and WORM credentials' do
    exporter = compose_config.dig('services', 'audit-exporter-prod')
    web_environment = compose_config.dig('services', 'web-prod', 'environment')

    expect(exporter['command']).to eq('bin/audit-exporter')
    expect(exporter.dig('environment', 'DATABASE_ROLE')).to eq('med_tracker_audit_exporter')
    expect(exporter.dig('environment', 'DATABASE_URL')).to include('medtracker_audit_exporter:')
    expect(exporter['environment']).to include(
      'AUDIT_WORM_BUCKET', 'AUDIT_WORM_EXPECTED_OWNER', 'AUDIT_SIGNING_KEY_ID', 'AUDIT_SIGNING_PRIVATE_KEY',
      'AWS_ACCESS_KEY_ID', 'AWS_SECRET_ACCESS_KEY'
    )
    expect(web_environment).not_to include(
      'AUDIT_WORM_BUCKET', 'AUDIT_SIGNING_PRIVATE_KEY', 'AWS_ACCESS_KEY_ID', 'AWS_SECRET_ACCESS_KEY'
    )
  end

  it 'does not grant the audit exporter login an application or owner role' do
    expect(init_roles_sql).to include('GRANT med_tracker_audit_exporter TO medtracker_audit_exporter;')
    expect(init_roles_sql).not_to include('GRANT med_tracker_app TO medtracker_audit_exporter;')
    expect(init_roles_sql).not_to include('GRANT med_tracker_owner TO medtracker_audit_exporter;')
  end

  it 'runs audit verification without web, exporter, owner, or clinical database privileges', :aggregate_failures do
    verifier = compose_config.dig('services', 'audit-verifier-prod')

    expect(verifier.dig('environment', 'DATABASE_ROLE')).to eq('med_tracker_audit_verifier')
    expect(verifier.dig('environment', 'DATABASE_URL')).to include('medtracker_audit_verifier:')
    expect(verifier['environment']).to include('SCOPE', 'FORMAT', 'HOUSEHOLD_ID', 'FROM', 'TO')
    expect(init_roles_sql).to include('GRANT med_tracker_audit_verifier TO medtracker_audit_verifier;')
    expect(init_roles_sql).not_to include('GRANT med_tracker_owner TO medtracker_audit_verifier;')
    expect(init_roles_sql).not_to include('GRANT med_tracker_app TO medtracker_audit_verifier;')
  end

  it 'bootstraps the deployed runtime role without owner or bypassrls privileges' do
    expect(init_roles_sql).to include(
      'ALTER ROLE med_tracker_app NOLOGIN NOSUPERUSER NOCREATEROLE NOCREATEDB NOREPLICATION NOBYPASSRLS;'
    )
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

  context 'with CI database login routing' do
    it 'bootstraps roles before migrating through the owner-only login' do
      %w[test_non_system test_system mutation lighthouse].each do |job_name|
        expect_ci_database_routing(job_name)
      end
    end

    it 'uses bootstrap access only for CI test harness steps' do
      ci_harness_steps.each do |job_name, step_name|
        step = ci_workflow.dig(job_name, 'steps').find { |candidate| candidate['name'] == step_name }

        expect(step.fetch('env')).to include(
          'DATABASE_URL' => ci_bootstrap_url,
          'DATABASE_ROLE' => nil
        )
      end
    end

    it 'scopes each database credential to only the step that consumes it' do
      %w[test_non_system test_system mutation lighthouse].each do |job_name|
        expect(ci_workflow.fetch(job_name).fetch('env')).not_to include(
          'BOOTSTRAP_DATABASE_URL', 'MIGRATION_DATABASE_URL', 'RUNTIME_DATABASE_URL', 'DATABASE_URL'
        )
        expect_ci_step_credentials(ci_workflow.fetch(job_name))
      end
    end

    it 'starts the CI Rails server through the runtime-only login' do
      server_step = ci_workflow.dig('lighthouse', 'steps').find { |step| step['name'] == 'Start Rails server' }

      expect(server_step.fetch('env')).to include(
        'DATABASE_URL' => ci_runtime_url,
        'DATABASE_ROLE' => 'med_tracker_app'
      )
    end
  end

  def expected_database_urls
    {
      'migrate-dev' => development_migration_urls,
      'web-dev' => development_runtime_urls,
      'migrate-test' => test_migration_urls,
      'web-test' => test_runtime_urls,
      'migrate-prod' => production_migration_urls,
      'web-prod' => production_runtime_urls
    }
  end

  def development_migration_urls
    {
      'DATABASE_URL' => 'postgresql://medtracker_migration:local_migration_only@db-dev:5432/medtracker',
      'SOLID_QUEUE_DATABASE_URL' =>
        'postgresql://medtracker_auxiliary:local_auxiliary_only@db-dev:5432/medtracker_queue'
    }
  end

  def development_runtime_urls
    development_migration_urls.merge(
      'DATABASE_URL' => 'postgresql://medtracker_runtime:local_runtime_only@db-dev:5432/medtracker'
    )
  end

  def test_migration_urls
    { 'DATABASE_URL' => 'postgresql://medtracker_migration:local_migration_only@db-test:5432/medtracker' }
  end

  def test_runtime_urls
    { 'DATABASE_URL' => 'postgresql://medtracker_runtime:local_runtime_only@db-test:5432/medtracker' }
  end

  def production_migration_urls
    {
      'DATABASE_URL' => 'postgresql://medtracker_migration:local_migration_only@db-prod:5432/medtracker',
      'SOLID_QUEUE_DATABASE_URL' => auxiliary_production_url('queue'),
      'SOLID_CACHE_DATABASE_URL' => auxiliary_production_url('cache'),
      'SOLID_CABLE_DATABASE_URL' => auxiliary_production_url('cable')
    }
  end

  def production_runtime_urls
    production_migration_urls.merge(
      'DATABASE_URL' => 'postgresql://medtracker_runtime:local_runtime_only@db-prod:5432/medtracker'
    )
  end

  def auxiliary_production_url(database)
    "postgresql://medtracker_auxiliary:local_auxiliary_only@db-prod:5432/medtracker_production_#{database}"
  end

  def expect_primary_login_memberships
    expect(init_roles_sql).to include(*expected_primary_membership_statements)
    expect(init_roles_sql).not_to include(
      'GRANT med_tracker_owner TO medtracker_runtime',
      'GRANT med_tracker_app TO medtracker_migration',
      'GRANT med_tracker_owner TO medtracker;',
      'GRANT med_tracker_app TO medtracker;'
    )
  end

  def expected_primary_membership_statements
    [
      'GRANT med_tracker_app TO medtracker_runtime WITH ADMIN FALSE, INHERIT FALSE, SET TRUE;',
      'GRANT med_tracker_owner TO medtracker_migration WITH ADMIN FALSE, INHERIT FALSE, SET TRUE;',
      "WHERE member.rolname IN ('medtracker_auxiliary', 'medtracker_migration', 'medtracker_runtime')",
      '\\if :{?runtime_password}',
      '\\if :{?migration_password}',
      "PASSWORD %L NOSUPERUSER NOCREATEROLE NOCREATEDB NOREPLICATION NOBYPASSRLS', :'runtime_password'",
      "PASSWORD %L NOSUPERUSER NOCREATEROLE NOCREATEDB NOREPLICATION NOBYPASSRLS', :'migration_password'",
      'ALTER DEFAULT PRIVILEGES FOR ROLE med_tracker_owner IN SCHEMA public'
    ]
  end

  def init_multiple_databases
    Rails.root.join('compose/init-multiple-dbs.sh').read
  end

  def ci_workflow
    YAML.safe_load(Rails.root.join('.github/workflows/ci.yml').read, aliases: true).fetch('jobs')
  end

  def ci_harness_steps
    {
      'test_non_system' => 'Run non-browser tests',
      'test_system' => 'Run browser tests (shard ${{ matrix.shard }}/2)',
      'mutation' => 'Mutation test changed subjects (advisory)'
    }
  end

  def expect_ci_database_routing(job_name)
    job = ci_workflow.fetch(job_name)
    expect_ci_database_urls(job)
    expect_ci_database_steps(job)
  end

  def expect_ci_database_urls(job)
    expect(job.fetch('env')).not_to include(
      'BOOTSTRAP_DATABASE_URL', 'MIGRATION_DATABASE_URL', 'RUNTIME_DATABASE_URL', 'DATABASE_URL'
    )
  end

  def expect_ci_database_steps(job)
    steps = job.fetch('steps').index_by { |step| step['name'] }

    expect_ci_bootstrap_step(steps.fetch('Bootstrap database roles'))
    expect(steps.dig('Set up database', 'env')).to include(
      'DATABASE_URL' => ci_migration_url,
      'DATABASE_ROLE' => 'med_tracker_owner'
    )
  end

  def expect_ci_bootstrap_step(step)
    expect(step.fetch('run')).to eq('psql "$DATABASE_URL" --file compose/init-roles.sql')
    expect(step.fetch('env')).to eq('DATABASE_URL' => ci_bootstrap_url)
  end

  def expect_ci_step_credentials(job)
    job.fetch('steps').each do |step|
      environment = step.fetch('env', {})
      expected_url = ci_step_database_urls.fetch(step['name'], nil)

      if expected_url
        expect(environment.fetch('DATABASE_URL')).to eq(expected_url)
      else
        expect(environment.to_json).not_to include('medtracker_password', 'local_migration_only', 'local_runtime_only')
      end
    end
  end

  def ci_step_database_urls
    {
      'Bootstrap database roles' => ci_bootstrap_url,
      'Set up database' => ci_migration_url,
      'Precompile assets' => ci_runtime_url,
      'Run non-browser tests' => ci_bootstrap_url,
      'Run browser tests (shard ${{ matrix.shard }}/2)' => ci_bootstrap_url,
      'Mutation test changed subjects (advisory)' => ci_bootstrap_url,
      'Seed database' => ci_bootstrap_url,
      'Start Rails server' => ci_runtime_url
    }
  end

  def ci_bootstrap_url
    'postgres://${{ secrets.POSTGRES_USER }}:${{ secrets.POSTGRES_PASSWORD }}@localhost:5432/medtracker_test'
  end

  def ci_migration_url
    'postgres://medtracker_migration:local_migration_only@localhost:5432/medtracker_test'
  end

  def ci_runtime_url
    'postgres://medtracker_runtime:local_runtime_only@localhost:5432/medtracker_test'
  end
end

# frozen_string_literal: true

require 'rails_helper'
require 'erb'
require 'open3'
require 'pg'
require 'uri'

module DatabaseRoleConfig
end

RSpec.describe DatabaseRoleConfig do
  context 'with database.yml' do
    around do |example|
      database_environment = {
        'DATABASE_ROLE' => 'med_tracker_app',
        'DATABASE_URL' => 'postgresql://runtime@database/primary',
        'SOLID_QUEUE_DATABASE_URL' => 'postgresql://auxiliary@database/queue',
        'SOLID_CACHE_DATABASE_URL' => 'postgresql://auxiliary@database/cache',
        'SOLID_CABLE_DATABASE_URL' => 'postgresql://auxiliary@database/cable'
      }
      original_environment = ENV.to_h.slice(*database_environment.keys)
      ENV.update(database_environment)
      example.run
    ensure
      database_environment.each_key do |key|
        original_environment.key?(key) ? ENV[key] = original_environment[key] : ENV.delete(key)
      end
    end

    it 'applies DATABASE_ROLE to primary tenant database connections' do
      expect(config.dig('development', 'primary', 'variables', 'role')).to eq('med_tracker_app')
      expect(config.dig('test', 'variables', 'role')).to eq('med_tracker_app')
      expect(config.dig('production', 'primary', 'variables', 'role')).to eq('med_tracker_app')
    end

    it 'does not apply DATABASE_ROLE to development auxiliary databases' do
      expect(config.dig('development', 'queue', 'variables')).to be_nil
    end

    it 'does not apply DATABASE_ROLE to production auxiliary databases' do
      expect(config.dig('production', 'queue', 'variables')).to be_nil
      expect(config.dig('production', 'cache', 'variables')).to be_nil
      expect(config.dig('production', 'cable', 'variables')).to be_nil
    end

    it 'preserves distinct primary and auxiliary connection URLs' do
      expect(configured_urls).to eq(
        development_primary: 'postgresql://runtime@database/primary',
        development_queue: 'postgresql://auxiliary@database/queue',
        production_primary: 'postgresql://runtime@database/primary',
        production_queue: 'postgresql://auxiliary@database/queue',
        production_cache: 'postgresql://auxiliary@database/cache',
        production_cable: 'postgresql://auxiliary@database/cable'
      )
    end

    it 'does not derive auxiliary credentials from the primary URL' do
      expect(database_config_source).not_to include("ENV['DATABASE_URL']&.gsub")
    end

    it 'keeps database preparation from seeding through the migration login' do
      expect(config.dig('development', 'primary', 'seeds')).to be(false)
      expect(config.dig('test', 'seeds')).to be(false)
    end

    def config
      YAML.safe_load(ERB.new(database_config_source).result, aliases: true)
    end

    def configured_urls
      {
        development_primary: config.dig('development', 'primary', 'url'),
        development_queue: config.dig('development', 'queue', 'url'),
        production_primary: config.dig('production', 'primary', 'url'),
        production_queue: config.dig('production', 'queue', 'url'),
        production_cache: config.dig('production', 'cache', 'url'),
        production_cable: config.dig('production', 'cable', 'url')
      }
    end

    def database_config_source
      Rails.root.join('config/database.yml').read
    end
  end

  context 'with database login documentation' do
    let(:deployment_guide) { Rails.root.join('docs/deployment.md').read }
    let(:upgrade_guide) { Rails.root.join('docs/pre-0-5-database-upgrade.md').read }
    let(:testing_guide) { Rails.root.join('docs/testing.md').read }

    it 'routes existing deployments through distinct bootstrap, migration, and runtime credentials' do
      expect(deployment_guide).to include(
        'BOOTSTRAP_DATABASE_URL',
        'MIGRATION_DATABASE_URL',
        'RUNTIME_DATABASE_URL',
        'SOLID_QUEUE_DATABASE_URL'
      )
      expect(upgrade_guide).to include(
        'compose/init-roles.sql',
        'medtracker_migration',
        'medtracker_runtime',
        'WITH INHERIT FALSE, SET TRUE',
        'db:migrate'
      )
    end

    it 'documents the narrow bootstrap exception for fixture-backed tests' do
      expect(testing_guide).to include('test-runner', 'bootstrap credential', 'fixture')
      expect(testing_guide).to include('web-test', 'medtracker_runtime', 'migrate-test', 'medtracker_migration')
    end
  end

  context 'with live PostgreSQL login isolation' do
    let(:database_uri) { URI.parse(ENV.fetch('DATABASE_URL')) }

    it 'reapplies the bootstrap artifact with deployment-supplied passwords' do
      output, status = run_role_bootstrap

      expect(status).to be_success, output
    end

    it 'allows the runtime login to assume only the application role' do
      with_connection('medtracker_runtime', 'local_runtime_only') do |connection|
        expect_runtime_login_isolated(connection)
      end
    end

    it 'allows the migration login to assume only the owner role' do
      with_connection('medtracker_migration', 'local_migration_only') do |connection|
        expect_migration_login_isolated(connection)
      end
    end

    it 'denies the auxiliary login any connection to the primary database' do
      expect { connect_as('medtracker_auxiliary', 'local_auxiliary_only') }
        .to raise_error(PG::ConnectionBad, /permission denied for database/)
    end

    def run_role_bootstrap
      Open3.capture2e(
        'psql', ENV.fetch('DATABASE_URL'),
        '--set=runtime_password=local_runtime_only',
        '--set=migration_password=local_migration_only',
        '--set=auxiliary_password=local_auxiliary_only',
        '--file', Rails.root.join('compose/init-roles.sql').to_s
      )
    end

    def expect_runtime_login_isolated(connection)
      expect(login_attributes(connection)).to eq(safe_login_attributes)
      expect(role_memberships(connection, 'medtracker_runtime')).to eq(
        [{ 'role_name' => 'med_tracker_app', 'inherit_option' => 'false', 'set_option' => 'true' }]
      )
      expect { connection.exec('SET ROLE med_tracker_owner') }.to raise_error(PG::InsufficientPrivilege)

      connection.exec('SET ROLE med_tracker_app')
      expect_runtime_access(connection)
    end

    def expect_migration_login_isolated(connection)
      expect(login_attributes(connection)).to eq(safe_login_attributes)
      expect_migration_membership(connection)
      expect_migration_access(connection)
    end

    def expect_migration_membership(connection)
      expect(role_memberships(connection, 'medtracker_migration')).to eq(
        [{ 'role_name' => 'med_tracker_owner', 'inherit_option' => 'false', 'set_option' => 'true' }]
      )
    end

    def expect_migration_access(connection)
      expect { connection.exec('SET ROLE med_tracker_app') }.to raise_error(PG::InsufficientPrivilege)

      connection.exec('SET ROLE med_tracker_owner')
      expect(connection.exec('SELECT current_user').getvalue(0, 0)).to eq('med_tracker_owner')
    end

    def expect_runtime_access(connection)
      expect_runtime_identity(connection)
      expect_runtime_data_access(connection)
    end

    def expect_runtime_identity(connection)
      expect(connection.exec('SELECT current_user').getvalue(0, 0)).to eq('med_tracker_app')
      expect(connection.exec("SELECT has_schema_privilege(current_user, 'public', 'CREATE')").getvalue(0, 0))
        .to eq('f')
    end

    def expect_runtime_data_access(connection)
      expect(connection.exec('SELECT COUNT(*) FROM schema_migrations').ntuples).to eq(1)
      expect(connection.exec('SELECT med_tracker.current_household_id()').getvalue(0, 0)).to be_nil
    end

    def login_attributes(connection)
      connection.exec(<<~SQL.squish).first
        SELECT rolsuper::text,
               rolcreaterole::text,
               rolcreatedb::text,
               rolreplication::text,
               rolbypassrls::text
        FROM pg_roles
        WHERE rolname = session_user
      SQL
    end

    def safe_login_attributes
      {
        'rolsuper' => 'false',
        'rolcreaterole' => 'false',
        'rolcreatedb' => 'false',
        'rolreplication' => 'false',
        'rolbypassrls' => 'false'
      }
    end

    def with_connection(user, password)
      connection = connect_as(user, password)
      yield connection
    ensure
      connection&.close
    end

    def connect_as(user, password)
      PG.connect(
        host: database_uri.host,
        port: database_uri.port,
        dbname: database_uri.path.delete_prefix('/'),
        user:,
        password:
      )
    end

    def role_memberships(connection, member_name)
      connection.exec_params(<<~SQL.squish, [member_name]).to_a
        SELECT granted.rolname AS role_name,
               memberships.inherit_option::text,
               memberships.set_option::text
        FROM pg_auth_members memberships
        JOIN pg_roles granted ON granted.oid = memberships.roleid
        JOIN pg_roles member ON member.oid = memberships.member
        WHERE member.rolname = $1
        ORDER BY granted.rolname
      SQL
    end
  end
end

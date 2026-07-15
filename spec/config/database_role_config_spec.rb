# frozen_string_literal: true

require 'rails_helper'
require 'erb'
require 'open3'
require 'pg'
require 'securerandom'
require 'tempfile'
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
    let(:root_testing_guide) { Rails.root.join('TESTING.md').read }

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

    it 'uses environment-sourced secrets in fish-compatible bootstrap commands' do
      expect(deployment_guide).to include('```fish', 'RUNTIME_DATABASE_PASSWORD', '--file compose/init-roles.sql')
      expect(upgrade_guide).to include('```fish', 'RUNTIME_DATABASE_PASSWORD', '--file compose/init-roles.sql')
      expect(deployment_guide).not_to match(/--set=.*password/i)
      expect(upgrade_guide).not_to match(/--set=.*password/i)
    end

    it 'provides one fail-fast secret-safe CNPG bootstrap flow' do
      expect(upgrade_guide).to include('kubectl port-forward', 'PGPASSFILE', '--file compose/init-roles.sql')
      expect(upgrade_guide).to include(
        'set namespace your-namespace',
        'set rw_service your-cluster-rw',
        'set database_name medtracker',
        'set database_admin database-admin'
      )
      expect(upgrade_guide).not_to match(/```fish.*?<[^>]+>.*?```/m)
      expect(upgrade_guide).not_to include('Paste the bootstrap SQL')
    end

    it 'documents the populated auxiliary database ownership upgrade' do
      expect(deployment_guide).to include('existing auxiliary databases', 'compose/init-multiple-dbs.sh')
      expect(upgrade_guide).to include('existing auxiliary databases', 'compose/init-multiple-dbs.sh')
    end

    it 'keeps the root testing guide on the separated runtime login' do
      expect(root_testing_guide).to include(
        'postgresql://medtracker_runtime:local_runtime_only@db-test:5432/medtracker'
      )
      expect(root_testing_guide).not_to include(
        'DATABASE_URL: postgresql://medtracker:medtracker_password@db-test:5432/medtracker'
      )
    end
  end

  context 'with live PostgreSQL login isolation' do
    let(:database_uri) { URI.parse(ENV.fetch('DATABASE_URL')) }

    it 'reapplies the bootstrap artifact with deployment-supplied passwords' do
      output, status = run_role_bootstrap

      expect(status).to be_success, output
    end

    it 'returns nonzero when a middle bootstrap statement fails' do
      Tempfile.create(['role-bootstrap-fail-fast', '.sql']) do |file|
        file.write(fail_fast_probe_sql)
        file.flush

        output, status = Open3.capture2e('psql', ENV.fetch('DATABASE_URL'), '--file', file.path)

        expect(status).not_to be_success, output
      end
    end

    it 'repairs legacy role attributes, memberships, and public schema creation' do
      configure_unsafe_legacy_roles

      output, status = run_role_bootstrap

      expect(status).to be_success, output
      expect_repaired_role_contract
    ensure
      restore_role_contract
    end

    it 'upgrades a populated auxiliary database without data loss' do
      database_name = "medtracker_auxiliary_upgrade_#{SecureRandom.hex(5)}"
      create_legacy_auxiliary_database(database_name)

      output, status = run_auxiliary_upgrade(database_name)

      expect(status).to be_success, output
      expect_upgraded_auxiliary_database(database_name)
    ensure
      drop_database(database_name) if database_name
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
        {
          'RUNTIME_DATABASE_PASSWORD' => 'local_runtime_only',
          'MIGRATION_DATABASE_PASSWORD' => 'local_migration_only',
          'AUXILIARY_DATABASE_PASSWORD' => 'local_auxiliary_only'
        },
        'psql', ENV.fetch('DATABASE_URL'),
        '--file', Rails.root.join('compose/init-roles.sql').to_s
      )
    end

    def fail_fast_probe_sql
      directive = Rails.root.join('compose/init-roles.sql').read[/^\\set ON_ERROR_STOP on$/]
      [directive, 'SELECT 1;', 'SELECT medtracker_missing_bootstrap_function();', 'SELECT 2;'].compact.join("\n")
    end

    def configure_unsafe_legacy_roles
      bootstrap_connection.exec('CREATE ROLE medtracker_indirect_owner NOLOGIN')
      bootstrap_connection.exec('GRANT med_tracker_owner TO medtracker_indirect_owner WITH SET TRUE')
      bootstrap_connection.exec('GRANT medtracker_indirect_owner TO medtracker_runtime WITH SET TRUE')
      bootstrap_connection.exec(
        'GRANT med_tracker_app TO medtracker_runtime WITH ADMIN TRUE, INHERIT TRUE, SET TRUE'
      )
      bootstrap_connection.exec(
        'GRANT med_tracker_app TO medtracker_migration WITH ADMIN TRUE, INHERIT TRUE, SET TRUE'
      )
      bootstrap_connection.exec(
        'ALTER ROLE med_tracker_owner CREATEROLE CREATEDB REPLICATION BYPASSRLS'
      )
      bootstrap_connection.exec('GRANT CREATE ON SCHEMA public TO PUBLIC')
    end

    def expect_repaired_role_contract
      expect_safe_group_role
      expect_isolated_migration_membership
      expect_legacy_access_paths_removed
    end

    def expect_safe_group_role
      expect(group_role_attributes('med_tracker_owner')).to eq(safe_group_role_attributes)
    end

    def expect_isolated_migration_membership
      expect(role_memberships(bootstrap_connection, 'medtracker_migration')).to eq(
        [{ 'role_name' => 'med_tracker_owner', 'admin_option' => 'false',
           'inherit_option' => 'false', 'set_option' => 'true' }]
      )
    end

    def expect_legacy_access_paths_removed
      expect(bootstrap_connection.exec(
        "SELECT pg_has_role('medtracker_runtime', 'med_tracker_owner', 'SET')::text"
      ).getvalue(0, 0)).to eq('false')
      expect(bootstrap_connection.exec(
        "SELECT has_schema_privilege('public', 'public', 'CREATE')::text"
      ).getvalue(0, 0)).to eq('false')
    end

    def restore_role_contract
      connection = bootstrap_connection
      connection.exec('REVOKE med_tracker_app FROM medtracker_migration')
      connection.exec('REVOKE medtracker_indirect_owner FROM medtracker_runtime')
      connection.exec('DROP ROLE IF EXISTS medtracker_indirect_owner')
      connection.exec(
        'GRANT med_tracker_app TO medtracker_runtime WITH ADMIN FALSE, INHERIT FALSE, SET TRUE'
      )
      connection.exec(
        'ALTER ROLE med_tracker_owner NOLOGIN NOSUPERUSER NOCREATEROLE NOCREATEDB NOREPLICATION NOBYPASSRLS'
      )
      connection.exec('REVOKE CREATE ON SCHEMA public FROM PUBLIC')
    end

    def create_legacy_auxiliary_database(database_name)
      bootstrap_connection.exec(
        "CREATE DATABASE #{PG::Connection.quote_ident(database_name)} OWNER #{PG::Connection.quote_ident(database_uri.user)}"
      )
      with_database_connection(database_uri.user, database_uri.password, database_name) do |connection|
        connection.exec('CREATE TABLE legacy_rows (value text NOT NULL)')
        connection.exec("INSERT INTO legacy_rows (value) VALUES ('preserved')")
        connection.exec('GRANT CREATE ON SCHEMA public TO PUBLIC')
      end
    end

    def run_auxiliary_upgrade(database_name)
      Open3.capture2e(
        {
          'POSTGRES_MULTIPLE_DATABASES' => database_name,
          'POSTGRES_USER' => database_uri.user,
          'PGHOST' => database_uri.host,
          'PGPORT' => database_uri.port.to_s,
          'PGPASSWORD' => database_uri.password
        },
        Rails.root.join('compose/init-multiple-dbs.sh').to_s
      )
    end

    def expect_upgraded_auxiliary_database(database_name)
      with_database_connection('medtracker_auxiliary', 'local_auxiliary_only', database_name) do |connection|
        expect_auxiliary_data_preserved(connection)
        expect_auxiliary_ownership(connection)
        expect_public_schema_locked(connection)
      end
      expect(database_owner(database_name)).to eq('medtracker_auxiliary')
    end

    def expect_auxiliary_data_preserved(connection)
      expect(connection.exec('SELECT value FROM legacy_rows').getvalue(0, 0)).to eq('preserved')
    end

    def expect_auxiliary_ownership(connection)
      expect(object_owner(connection, 'legacy_rows')).to eq('medtracker_auxiliary')
      expect(schema_owner(connection, 'public')).to eq('medtracker_auxiliary')
    end

    def expect_public_schema_locked(connection)
      expect(connection.exec(
        "SELECT has_schema_privilege('public', 'public', 'CREATE')::text"
      ).getvalue(0, 0)).to eq('false')
    end

    def drop_database(database_name)
      bootstrap_connection.exec_params(
        'SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = $1', [database_name]
      )
      bootstrap_connection.exec("DROP DATABASE IF EXISTS #{PG::Connection.quote_ident(database_name)}")
    end

    def bootstrap_connection
      @bootstrap_connection ||= PG.connect(ENV.fetch('DATABASE_URL'))
    end

    def group_role_attributes(role_name)
      bootstrap_connection.exec_params(<<~SQL.squish, [role_name]).first
        SELECT rolcanlogin::text,
               rolsuper::text,
               rolcreaterole::text,
               rolcreatedb::text,
               rolreplication::text,
               rolbypassrls::text
        FROM pg_roles
        WHERE rolname = $1
      SQL
    end

    def safe_group_role_attributes
      safe_login_attributes.merge('rolcanlogin' => 'false')
    end

    def database_owner(database_name)
      bootstrap_connection.exec_params(<<~SQL.squish, [database_name]).getvalue(0, 0)
        SELECT owner.rolname
        FROM pg_database database
        JOIN pg_roles owner ON owner.oid = database.datdba
        WHERE database.datname = $1
      SQL
    end

    def object_owner(connection, object_name)
      connection.exec_params(<<~SQL.squish, [object_name]).getvalue(0, 0)
        SELECT owner.rolname
        FROM pg_class object
        JOIN pg_roles owner ON owner.oid = object.relowner
        WHERE object.relname = $1
      SQL
    end

    def schema_owner(connection, schema_name)
      connection.exec_params(<<~SQL.squish, [schema_name]).getvalue(0, 0)
        SELECT owner.rolname
        FROM pg_namespace schema
        JOIN pg_roles owner ON owner.oid = schema.nspowner
        WHERE schema.nspname = $1
      SQL
    end

    def expect_runtime_login_isolated(connection)
      expect(login_attributes(connection)).to eq(safe_login_attributes)
      expect(role_memberships(connection, 'medtracker_runtime')).to eq(
        [{ 'role_name' => 'med_tracker_app', 'admin_option' => 'false',
           'inherit_option' => 'false', 'set_option' => 'true' }]
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
        [{ 'role_name' => 'med_tracker_owner', 'admin_option' => 'false',
           'inherit_option' => 'false', 'set_option' => 'true' }]
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
      connect_to_database(user, password, database_uri.path.delete_prefix('/'))
    end

    def with_database_connection(user, password, database_name)
      connection = connect_to_database(user, password, database_name)
      yield connection
    ensure
      connection&.close
    end

    def connect_to_database(user, password, database_name)
      PG.connect(
        host: database_uri.host,
        port: database_uri.port,
        dbname: database_name,
        user:,
        password:
      )
    end

    def role_memberships(connection, member_name)
      connection.exec_params(<<~SQL.squish, [member_name]).to_a
        SELECT granted.rolname AS role_name,
               memberships.admin_option::text,
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

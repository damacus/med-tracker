# frozen_string_literal: true

require 'open3'
require 'rails_helper'
require 'uri'

FreshSchemaRowLevelSecurity = Data.define(:database_name)

RSpec.describe FreshSchemaRowLevelSecurity do
  let(:database_name) { "fresh_schema_rls_#{SecureRandom.hex(8)}" }
  let(:connection_parameters) { ActiveRecord::Base.connection.raw_connection.conninfo_hash.compact_blank }
  let(:admin_connection) { PG.connect(connection_parameters.merge(dbname: 'postgres')) }
  let(:connections) do
    {
      database: PG.connect(disposable_database_parameters),
      runtime: PG.connect(disposable_database_parameters),
      owner: PG.connect(disposable_database_parameters)
    }
  end

  after do
    close_disposable_connections
  ensure
    drop_database!
  end

  context 'when runtime roles are present' do
    let(:seeded_records) { seed_records! }

    before do
      create_database!
      load_schema!
      seeded_records
    end

    it 'restores the household RLS and runtime role contract after db:schema:load' do
      expect_household_policies!
      expect_people_login_lookup_policy!
      expect_role_convergence!
      expect_runtime_isolation!
      expect_owner_role_can_manage_a_matching_medication_take!
    end
  end

  context 'when runtime roles are absent' do
    let(:hidden_runtime_roles) { {} }

    before do
      hide_runtime_roles!
      create_database!
      load_schema!
    end

    after do
      restore_runtime_roles!
    end

    it 'loads the schema without the role-dependent login lookup policy' do
      expect(runtime_roles_present?).to be(false)
      expect(people_login_lookup_policy).to be_nil
    end
  end

  private

  def create_database!
    admin_connection.exec("CREATE DATABASE #{admin_connection.escape_identifier(database_name)}")
  end

  def load_schema!
    _output, error, status = Open3.capture3(
      { 'DATABASE_URL' => disposable_database_url, 'DATABASE_ROLE' => nil },
      'bin/rails', 'db:schema:load', chdir: Rails.root
    )

    expect(status).to be_success, error
  end

  def close_disposable_connections
    connections.each_value(&:close)
  end

  def drop_database!
    admin_connection.exec_params(
      'SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = $1 AND pid <> pg_backend_pid()',
      [database_name]
    )
    admin_connection.exec("DROP DATABASE IF EXISTS #{admin_connection.escape_identifier(database_name)}")
  ensure
    admin_connection.close
  end

  def disposable_database_parameters
    connection_parameters.merge(dbname: database_name)
  end

  def disposable_database_url
    uri = URI.parse(ActiveRecord::Base.connection_db_config.url)
    uri.path = "/#{database_name}"
    uri.to_s
  end

  def database_connection
    connections.fetch(:database)
  end

  def runtime_connection
    connections.fetch(:runtime)
  end

  def owner_connection
    connections.fetch(:owner)
  end

  def expect_household_policies!
    expect(missing_household_tables).to be_empty
    expect(incomplete_household_policies).to be_empty
  end

  def missing_household_tables
    SchemaInventory.household_owned_tables.reject { |table_name| table_exists?(table_name) }
  end

  def table_exists?(table_name)
    database_connection.exec_params('SELECT to_regclass($1)', [table_name]).getvalue(0, 0)
  end

  def incomplete_household_policies
    SchemaInventory.household_owned_tables.reject do |table_name|
      forced_rls?(table_name) && household_policy_for(table_name)
    end
  end

  def forced_rls?(table_name)
    row = database_connection.exec_params(<<~SQL.squish, [table_name]).first
      SELECT relrowsecurity, relforcerowsecurity FROM pg_class WHERE oid = $1::regclass
    SQL
    row == { 'relrowsecurity' => 't', 'relforcerowsecurity' => 't' }
  end

  def household_policy_for(table_name)
    row = database_connection.exec_params(<<~SQL.squish, [table_name]).first
      SELECT qual, with_check FROM pg_policies
      WHERE schemaname = 'public' AND tablename = $1 AND policyname = 'household_tenant_isolation'
    SQL
    row if row&.fetch('qual') && row.fetch('with_check')
  end

  def expect_people_login_lookup_policy!
    policy = people_login_lookup_policy

    expect(policy).to include('roles' => '{med_tracker_app}')
    expect(policy.fetch('qual')).to include('account_id IS NOT NULL')
  end

  def people_login_lookup_policy
    database_connection.exec(<<~SQL.squish).first
      SELECT roles, qual FROM pg_policies
      WHERE schemaname = 'public' AND tablename = 'people' AND policyname = 'people_account_login_lookup'
    SQL
  end

  def hide_runtime_roles!
    runtime_roles.each do |role|
      hidden_role = "#{role}_absent_#{SecureRandom.hex(8)}"
      rename_role!(role, hidden_role)
      hidden_runtime_roles[role] = hidden_role
    end
  end

  def restore_runtime_roles!
    hidden_runtime_roles.each do |role, hidden_role|
      rename_role!(hidden_role, role)
    end
  end

  def runtime_roles_present?
    runtime_roles.all? do |role|
      database_connection.exec_params('SELECT to_regrole($1)', [role]).getvalue(0, 0)
    end
  end

  def runtime_roles
    %w[med_tracker_owner med_tracker_app]
  end

  def rename_role!(from, to)
    admin_connection.exec(
      "ALTER ROLE #{admin_connection.escape_identifier(from)} RENAME TO #{admin_connection.escape_identifier(to)}"
    )
  end

  def expect_role_convergence!
    expect(incorrect_table_owners).to be_empty
    expect(missing_runtime_table_privileges).to be_empty
    expect(unusable_runtime_sequences).to be_empty
    expect_login_can_assume_runtime_roles!
    expect_default_table_privileges!
  end

  def incorrect_table_owners
    database_connection.exec(<<~SQL.squish).to_a
      SELECT c.relname FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
      WHERE n.nspname = 'public' AND c.relkind IN ('r', 'p', 'v', 'm')
        AND c.relname NOT IN ('ar_internal_metadata', 'schema_migrations')
        AND pg_get_userbyid(c.relowner) <> 'med_tracker_owner' ORDER BY c.relname
    SQL
  end

  def missing_runtime_table_privileges
    database_connection.exec(<<~SQL.squish).to_a
      SELECT c.relname FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
      WHERE n.nspname = 'public' AND c.relkind IN ('r', 'p', 'v', 'm')
        AND c.relname NOT IN ('ar_internal_metadata', 'schema_migrations')
        AND NOT has_table_privilege('med_tracker_app', c.oid, 'SELECT, INSERT, UPDATE, DELETE')
      ORDER BY c.relname
    SQL
  end

  def unusable_runtime_sequences
    database_connection.exec(<<~SQL.squish).to_a
      WITH sequences AS MATERIALIZED (
        SELECT c.oid, c.relname FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
        WHERE n.nspname = 'public' AND c.relkind = 'S'
      ) SELECT relname FROM sequences
      WHERE NOT has_sequence_privilege('med_tracker_app', oid, 'USAGE, SELECT') ORDER BY relname
    SQL
  end

  def expect_login_can_assume_runtime_roles!
    roles = database_connection.exec(<<~SQL.squish).first
      SELECT pg_has_role(session_user, 'med_tracker_owner', 'member') AS owner,
             pg_has_role(session_user, 'med_tracker_app', 'member') AS app
    SQL

    expect(roles).to eq('owner' => 't', 'app' => 't')
  end

  def expect_default_table_privileges!
    table_name = 'fresh_schema_default_privileges'
    owner_connection.exec('BEGIN')
    owner_connection.exec('SET LOCAL ROLE med_tracker_owner')
    owner_connection.exec("CREATE TABLE #{table_name} (id bigint)")
    owner_connection.exec('COMMIT')

    expect(runtime_has_table_privileges?(table_name)).to be(true)
  end

  def runtime_has_table_privileges?(table_name)
    database_connection.exec_params(
      "SELECT has_table_privilege('med_tracker_app', $1, 'SELECT, INSERT, UPDATE, DELETE')", [table_name]
    ).getvalue(0, 0) == 't'
  end

  def seed_records!
    household_a = insert_household('A')
    household_b = insert_household('B')
    location_a, medication_take_a = seed_household_a!(household_a)
    location_b = seed_location!(household_b, 'Fresh schema B location')

    { household_a:, household_b:, location_a:, location_b:, medication_take_a: }
  end

  def insert_household(label)
    insert_and_return_id(
      'households', %w[name slug timezone], ["Fresh schema #{label}", "fresh-schema-#{label.downcase}", 'UTC']
    )
  end

  def seed_household_a!(household_id)
    set_household_context!(household_id)
    location_id = seed_location!(household_id, 'Fresh schema A location')
    person_id = insert_and_return_id('people', %w[household_id name], [household_id, 'Fresh schema A person'])
    medication_id = insert_and_return_id(
      'medications', %w[household_id location_id name], [household_id, location_id, 'Fresh schema medication']
    )
    person_medication_id = insert_and_return_id(
      'person_medications',
      %w[household_id medication_id person_id position],
      [household_id, medication_id, person_id, 1]
    )
    [location_id, seed_medication_take!(household_id, person_medication_id, location_id, medication_id)]
  end

  def seed_location!(household_id, name)
    insert_and_return_id('locations', %w[household_id name], [household_id, name])
  end

  def seed_medication_take!(household_id, person_medication_id, location_id, medication_id)
    insert_and_return_id(
      'medication_takes',
      %w[household_id person_medication_id taken_from_location_id taken_from_medication_id],
      [household_id, person_medication_id, location_id, medication_id]
    )
  end

  def insert_and_return_id(table_name, columns, values)
    placeholders = (1..columns.length).map { |index| "$#{index}" }
    timestamp_columns = %w[created_at updated_at]
    timestamp_values = %w[CURRENT_TIMESTAMP CURRENT_TIMESTAMP]
    sql = <<~SQL.squish
      INSERT INTO #{table_name} (#{(columns + timestamp_columns).join(', ')})
      VALUES (#{(placeholders + timestamp_values).join(', ')}) RETURNING id
    SQL

    database_connection.exec_params(sql, values).getvalue(0, 0).to_i
  end

  def set_household_context!(household_id)
    database_connection.exec_params(
      "SELECT set_config('med_tracker.current_household_id', $1, false)", [household_id.to_s]
    )
  end

  def expect_runtime_isolation!
    expect_missing_context_to_default_deny!
    expect_household_context_to_isolate!
    expect_wrong_household_writes_to_fail!
    expect_location_b_to_remain_unchanged!
  end

  def expect_missing_context_to_default_deny!
    connection = PG.connect(disposable_database_parameters)
    with_runtime_role(connection, 'med_tracker_app') do
      expect_hidden_location(connection)
      expect_hidden_location_cannot_mutate(connection)
    end
  ensure
    connection&.close
  end

  def expect_household_context_to_isolate!
    with_runtime_role(runtime_connection, 'med_tracker_app', seeded_records.fetch(:household_a)) do
      expect_visible_location_can_update!
      expect_cross_household_location_is_hidden!
    end
  end

  def expect_wrong_household_writes_to_fail!
    expect_wrong_household_insert_to_fail!
    expect_rehome_to_fail!
  end

  def expect_wrong_household_insert_to_fail!
    with_runtime_role(runtime_connection, 'med_tracker_app', seeded_records.fetch(:household_a)) do
      expect do
        runtime_connection.exec_params(<<~SQL.squish, [seeded_records.fetch(:household_b), 'Rejected location'])
          INSERT INTO locations (household_id, name, created_at, updated_at)
          VALUES ($1, $2, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
        SQL
      end.to raise_error(PG::InsufficientPrivilege, /row-level security policy/)
    end
  end

  def expect_rehome_to_fail!
    with_runtime_role(runtime_connection, 'med_tracker_app', seeded_records.fetch(:household_a)) do
      expect do
        runtime_connection.exec_params(
          'UPDATE locations SET household_id = $1 WHERE id = $2',
          [seeded_records.fetch(:household_b), seeded_records.fetch(:location_a)]
        )
      end.to raise_error(PG::InsufficientPrivilege, /row-level security policy/)
    end
  end

  def expect_location_b_to_remain_unchanged!
    name = database_connection.exec_params(
      'SELECT name FROM locations WHERE id = $1', [seeded_records.fetch(:location_b)]
    ).getvalue(0, 0)
    expect(name).to eq('Fresh schema B location')
  end

  def expect_hidden_location(connection)
    expect(select_location(connection, seeded_records.fetch(:location_a))).to eq(0)
  end

  def expect_hidden_location_cannot_mutate(connection)
    expect(update_location(connection, 'Hidden', seeded_records.fetch(:location_a))).to eq(0)
    expect(delete_location(connection, seeded_records.fetch(:location_a))).to eq(0)
  end

  def expect_visible_location_can_update!
    expect(select_location(runtime_connection, seeded_records.fetch(:location_a))).to eq(1)
    expect(update_location(runtime_connection, 'Fresh schema A updated', seeded_records.fetch(:location_a))).to eq(1)
  end

  def expect_cross_household_location_is_hidden!
    expect_cross_household_location_to_be_hidden!
    expect_cross_household_location_cannot_mutate!
  end

  def expect_cross_household_location_to_be_hidden!
    expect(select_location(runtime_connection, seeded_records.fetch(:location_b))).to eq(0)
  end

  def expect_cross_household_location_cannot_mutate!
    expect(update_location(runtime_connection, 'Cross household', seeded_records.fetch(:location_b))).to eq(0)
    expect(delete_location(runtime_connection, seeded_records.fetch(:location_b))).to eq(0)
  end

  def select_location(connection, location_id)
    connection.exec_params('SELECT id FROM locations WHERE id = $1', [location_id]).ntuples
  end

  def update_location(connection, name, location_id)
    connection.exec_params('UPDATE locations SET name = $1 WHERE id = $2', [name, location_id]).cmd_tuples
  end

  def delete_location(connection, location_id)
    connection.exec_params('DELETE FROM locations WHERE id = $1', [location_id]).cmd_tuples
  end

  def expect_owner_role_can_manage_a_matching_medication_take!
    with_runtime_role(owner_connection, 'med_tracker_owner', seeded_records.fetch(:household_a)) do
      expect(update_medication_take).to eq(1)
      expect(delete_medication_take).to eq(1)
    end
  end

  def update_medication_take
    owner_connection.exec_params(
      'UPDATE medication_takes SET taken_at = CURRENT_TIMESTAMP WHERE id = $1',
      [seeded_records.fetch(:medication_take_a)]
    ).cmd_tuples
  end

  def delete_medication_take
    owner_connection.exec_params(
      'DELETE FROM medication_takes WHERE id = $1', [seeded_records.fetch(:medication_take_a)]
    ).cmd_tuples
  end

  def with_runtime_role(connection, role, household_id = nil)
    connection.exec('BEGIN')
    connection.exec("SET LOCAL ROLE #{role}")
    set_local_household_context(connection, household_id) if household_id
    yield
  ensure
    connection.exec('ROLLBACK') if connection&.transaction_status != PG::PQTRANS_IDLE
  end

  def set_local_household_context(connection, household_id)
    connection.exec_params(
      "SELECT set_config('med_tracker.current_household_id', $1, true)", [household_id.to_s]
    )
  end
end

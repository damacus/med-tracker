# frozen_string_literal: true

require 'rails_helper'
require Rails.root.join('db/migrate/20260716120000_create_household_medication_take_purge_function')

RSpec.describe CreateHouseholdMedicationTakePurgeFunction do
  delegate :connection, to: :'ActiveRecord::Base'

  let(:operator) do
    Account.create!(email: "purge-function-#{SecureRandom.hex(4)}@example.test", status: :verified).tap do |account|
      PlatformAdmin.create!(account: account)
    end
  end
  let(:household) { create(:household) }

  before do
    household.update!(status: :archived, lifecycle_state: :purging, offboarded_at: Time.current)
  end

  it 'installs the fixed-path security definer owned by the migration login' do
    reinstall_function_as_migration_login

    expect(function_contract).to include(
      'prosecdef' => true,
      'proconfig' => 'search_path=pg_catalog',
      'lanname' => 'plpgsql',
      'owner' => connection.select_value('SELECT current_user'),
      'result_type' => 'bigint'
    )
    expect(function_contract.fetch('definition')).to include('public.households', 'public.medication_takes')
  end

  def reinstall_function_as_migration_login
    migration = described_class.new
    migration.migrate(:down)
    migration.migrate(:up)
  end

  def function_contract
    connection.select_one(<<~SQL.squish)
      SELECT p.prosecdef,
             array_to_string(p.proconfig, ',') AS proconfig,
             l.lanname,
             r.rolname AS owner,
             pg_get_function_result(p.oid) AS result_type,
             pg_get_functiondef(p.oid) AS definition
      FROM pg_proc p
      JOIN pg_namespace n ON n.oid = p.pronamespace
      JOIN pg_language l ON l.oid = p.prolang
      JOIN pg_roles r ON r.oid = p.proowner
      WHERE n.nspname = 'med_tracker'
        AND p.proname = 'purge_medication_takes'
        AND pg_get_function_identity_arguments(p.oid) = 'p_household_id bigint'
    SQL
  end

  it 'revokes PUBLIC execution and grants med_tracker_app execution' do
    privileges = connection.select_one(<<~SQL.squish)
      SELECT p.proacl::text AS acl,
             has_function_privilege(
               'med_tracker_app',
               'med_tracker.purge_medication_takes(bigint)',
               'EXECUTE'
             ) AS app_can_execute
      FROM pg_proc p
      JOIN pg_namespace n ON n.oid = p.pronamespace
      WHERE n.nspname = 'med_tracker'
        AND p.proname = 'purge_medication_takes'
    SQL

    expect(privileges.fetch('acl')).not_to match(/(?:\{|,)=X/)
    expect(privileges.fetch('app_can_execute')).to be(true)
  end

  it 'deletes only target rows, returns the exact count, and returns zero on retry' do
    target_takes = [create_take(household), create_take(household)]
    other_household = create(:household)
    other_take = create_take(other_household)

    set_context(household_id: household.id, account_id: operator.id)

    expect(call_function(household.id)).to eq(2)
    expect(call_function(household.id)).to eq(0)
    expect(MedicationTake.where(id: target_takes.map(&:id))).to be_empty
    expect(MedicationTake.where(id: other_take.id)).to exist
  end

  it 'executes as med_tracker_app through the existing shared login' do
    take = create_take(household)

    connection.transaction(requires_new: true) do
      connection.execute('SET LOCAL ROLE med_tracker_app')
      set_context(household_id: household.id, account_id: operator.id)

      expect(call_function(household.id)).to eq(1)
      expect(MedicationTake.where(id: take.id)).not_to exist
      raise ActiveRecord::Rollback
    end
  end

  it 'uses the stable invalid-target contract' do
    missing_id = Household.maximum(:id) + 10_000
    set_context(household_id: missing_id, account_id: operator.id)

    expect_contract('MT101', 'household purge target is invalid') { call_function(missing_id) }
  end

  it 'uses the stable invalid-lifecycle contract' do
    household.update!(lifecycle_state: :offboarded)
    set_context(household_id: household.id, account_id: operator.id)

    expect_contract('MT102', 'household purge lifecycle is invalid') { call_function(household.id) }
  end

  it 'uses the stable active-hold contract' do
    HouseholdRetentionHold.create!(
      household: household,
      approved_by_account: operator,
      reason: 'Protected test reason',
      review_on: 1.month.from_now.to_date,
      placed_at: Time.current
    )
    set_context(household_id: household.id, account_id: operator.id)

    expect_contract('MT103', 'household purge retention hold is active') { call_function(household.id) }
  end

  it 'uses the stable tenant-context contract' do
    other_household = create(:household)
    set_context(household_id: other_household.id, account_id: operator.id)

    expect_contract('MT104', 'household purge tenant context does not match target') do
      call_function(household.id)
    end
  end

  it 'uses the stable tenant-context contract for an oversized numeric setting' do
    set_context(household_id: '9' * 100, account_id: operator.id)

    expect_contract('MT104', 'household purge tenant context does not match target') do
      call_function(household.id)
    end
  end

  it 'uses the stable operator-context contract' do
    ordinary_account = Account.create!(
      email: "purge-function-ordinary-#{SecureRandom.hex(4)}@example.test",
      status: :verified
    )
    set_context(household_id: household.id, account_id: ordinary_account.id)

    expect_contract('MT105', 'household purge operator context is invalid') { call_function(household.id) }
  end

  it 'uses the stable operator-context contract for an oversized numeric setting' do
    operator
    set_context(household_id: household.id, account_id: '9' * 100)

    expect_contract('MT105', 'household purge operator context is invalid') { call_function(household.id) }
  end

  it 'restores transaction-local tenant and operator settings after success' do
    set_context(household_id: household.id, account_id: operator.id)

    call_function(household.id)

    expect(current_context).to eq([household.id.to_s, operator.id.to_s])
  end

  it 'restores transaction-local tenant and operator settings after failure' do
    household.update!(lifecycle_state: :offboarded)
    set_context(household_id: household.id, account_id: operator.id)
    connection.execute(<<~SQL.squish)
      DO $purge_context$
      BEGIN
        PERFORM med_tracker.purge_medication_takes(#{household.id});
      EXCEPTION
        WHEN SQLSTATE 'MT102' THEN NULL;
      END
      $purge_context$;
    SQL

    expect(current_context).to eq([household.id.to_s, operator.id.to_s])
  end

  def call_function(household_id)
    connection.select_value(
      ActiveRecord::Base.sanitize_sql_array(
        ['SELECT med_tracker.purge_medication_takes(?)', household_id]
      )
    ).to_i
  end

  def create_take(target_household)
    person = create(:person, household: target_household)
    medication = create(:medication, household: target_household)
    schedule = create(
      :schedule,
      household: target_household,
      person: person,
      medication: medication
    )
    create(:medication_take, :for_schedule, household: target_household, schedule: schedule)
  end

  def set_context(household_id:, account_id:)
    connection.execute("SELECT set_config('med_tracker.current_household_id', '#{household_id}', true)")
    connection.execute("SELECT set_config('med_tracker.current_account_id', '#{account_id}', true)")
  end

  def current_context
    connection.select_rows(<<~SQL.squish).sole
      SELECT current_setting('med_tracker.current_household_id', true),
             current_setting('med_tracker.current_account_id', true)
    SQL
  end

  def expect_contract(sqlstate, message, &)
    error = database_error(&)

    expect(error.cause.result.error_field(PG::Result::PG_DIAG_SQLSTATE)).to eq(sqlstate)
    expect(error.cause.message).to include(message)
  end

  def database_error(&)
    captured_error = nil
    expect do
      connection.transaction(requires_new: true, &)
    end.to raise_error(ActiveRecord::StatementInvalid) { |exception| captured_error = exception }

    captured_error
  end
end

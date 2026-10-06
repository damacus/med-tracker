# frozen_string_literal: true

require 'rails_helper'
require 'timeout'

RSpec.describe Households::LifecycleCutoffLock, '.with' do
  self.use_transactional_tests = false

  let(:workers) { [] }
  let(:records) { PaperTrail.request(enabled: false) { concurrency_records } }

  before do
    allow(Audit::Event).to receive(:record!)
    records
  end

  after do
    terminate_workers
    cleanup_records
  end

  it 'completes a dose inside the cutoff boundary before purging it' do
    outcome = dose_first_outcome

    expect(dose_first_state(outcome)).to eq(
      dose_succeeded: true,
      purge_completed: true,
      take_exists: false,
      household_purged: true
    )
  end

  it 'blocks a dose behind purge cutoff without blocking an unrelated household' do
    outcome = purge_first_outcome

    expect(purge_first_state(outcome)).to eq(
      unrelated_dose_succeeded: true,
      unrelated_take_exists: true,
      purge_completed: true,
      target_error: :household_unavailable,
      target_take_exists: false,
      target_stock_mutated: false
    )
  end

  def dose_first_outcome
    dose_state = start_paused_dose
    purge_worker = start_blocked_purge
    dose_state.fetch(:release) << true

    {
      dose_result: join_worker(dose_state.fetch(:worker)),
      purge_run: join_worker(purge_worker)
    }
  end

  def start_paused_dose
    inside_dose = Queue.new
    release_dose = Queue.new
    dose_service = paused_dose_service(inside_dose, release_dose)
    dose_worker = worker { record_dose(records.fetch(:target), service: dose_service) }
    wait_for(inside_dose, 'dose cutoff acquisition')
    { worker: dose_worker, release: release_dose }
  end

  def start_blocked_purge
    purge_pid = Queue.new
    purge_worker = worker_with_pid(purge_pid) { purge_target(records) }
    wait_for_advisory_wait(wait_for(purge_pid, 'purge database session'))
    purge_worker
  end

  def paused_dose_service(inside_dose, release_dose)
    MedicationAdministration::RecordDose.new.tap do |service|
      original_record_dose = service.method(:record_dose)
      allow(service).to receive(:record_dose) do |**arguments|
        inside_dose << true
        wait_for(release_dose, 'dose release')
        original_record_dose.call(**arguments)
      end
    end
  end

  def dose_first_state(outcome)
    dose_result = outcome.fetch(:dose_result)
    {
      dose_succeeded: dose_result.success,
      purge_completed: outcome.fetch(:purge_run).completed?,
      take_exists: MedicationTake.exists?(id: dose_result.take.id),
      household_purged: Household.find(records.dig(:target, :household_id)).purged?
    }
  end

  def purge_first_outcome
    purge_state = start_paused_purge
    target_dose_worker = start_blocked_target_dose
    unrelated_dose_worker = worker { record_dose(records.fetch(:unrelated)) }
    unrelated_result = join_worker(unrelated_dose_worker)
    purge_state.fetch(:release) << true

    purge_first_results(unrelated_result, purge_state, target_dose_worker)
  end

  def start_paused_purge
    inside_purge = Queue.new
    release_purge = Queue.new
    target_stock_mutations = Queue.new
    pause_purge(inside_purge, release_purge)
    track_target_stock_mutations(target_stock_mutations)
    purge_worker = worker { purge_target(records) }
    wait_for(inside_purge, 'purge cutoff acquisition')
    { worker: purge_worker, release: release_purge, stock_mutations: target_stock_mutations }
  end

  def start_blocked_target_dose
    dose_pid = Queue.new
    target_dose_worker = worker_with_pid(dose_pid) { record_dose(records.fetch(:target)) }
    wait_for_advisory_wait(wait_for(dose_pid, 'dose database session'))
    target_dose_worker
  end

  def purge_first_results(unrelated_result, purge_state, target_dose_worker)
    {
      unrelated_result: unrelated_result,
      purge_run: join_worker(purge_state.fetch(:worker)),
      target_result: join_worker(target_dose_worker),
      target_stock_mutations: purge_state.fetch(:stock_mutations)
    }
  end

  def pause_purge(inside_purge, release_purge)
    original_execute_purge = Households::Purger.method(:execute_purge!)
    allow(Households::Purger).to receive(:execute_purge!) do |*arguments|
      inside_purge << true
      wait_for(release_purge, 'purge release')
      original_execute_purge.call(*arguments)
    end
  end

  def track_target_stock_mutations(target_stock_mutations)
    allow(MedicationTakeStockDecrement).to receive(:new).and_wrap_original do |original, take|
      target_stock_mutations << take.id if take.household_id == records.dig(:target, :household_id)
      original.call(take)
    end
  end

  def purge_first_state(outcome)
    unrelated_result = outcome.fetch(:unrelated_result)
    {
      unrelated_dose_succeeded: unrelated_result.success,
      unrelated_take_exists: MedicationTake.exists?(id: unrelated_result.take.id),
      purge_completed: outcome.fetch(:purge_run).completed?,
      target_error: outcome.fetch(:target_result).error,
      target_take_exists: MedicationTake.exists?(schedule_id: records.dig(:target, :schedule_id)),
      target_stock_mutated: !outcome.fetch(:target_stock_mutations).empty?
    }
  end

  def concurrency_records
    token = SecureRandom.hex(6)
    operator = Account.create!(email: "purge-concurrency-#{token}@example.test", status: :verified)
    PlatformAdmin.create!(account: operator)
    target_household = create(:household, name: "Purge Concurrency #{token}", slug: "purge-concurrency-#{token}")
    unrelated_household = create(
      :household,
      name: "Unrelated Purge Concurrency #{token}",
      slug: "unrelated-purge-concurrency-#{token}"
    )

    {
      operator_id: operator.id,
      target: dose_records(target_household, "target-#{token}"),
      unrelated: dose_records(unrelated_household, "unrelated-#{token}")
    }
  end

  def dose_records(household, token)
    person = create(:person, household: household)
    medication = create(:medication, household: household, current_supply: 10, supply_at_last_restock: 10)
    schedule = create(:schedule, household: household, person: person, medication: medication)
    user = User.create!(
      person: person,
      email_address: "#{token}@example.test",
      password: 'password'
    )
    {
      household_id: household.id,
      schedule_id: schedule.id,
      medication_id: medication.id,
      user_id: user.id,
      user_email: user.email_address
    }
  end

  def worker(&)
    thread = Thread.new do
      ActiveRecord::Base.connection_pool.with_connection do
        PaperTrail.request(enabled: false, &)
      end
    rescue StandardError => e
      e
    end
    workers << thread
    thread
  end

  def worker_with_pid(pid_queue, &)
    worker do
      pid_queue << ActiveRecord::Base.connection.select_value('SELECT pg_backend_pid()').to_i
      yield
    end
  end

  def join_worker(worker)
    raise Timeout::Error, 'timed out waiting for concurrency worker' unless worker.join(thread_timeout)

    worker.value.tap { raise it if it.is_a?(StandardError) }
  end

  def wait_for(queue, description)
    Timeout.timeout(thread_timeout) { queue.pop }
  rescue Timeout::Error
    raise Timeout::Error, "timed out waiting for #{description}"
  end

  def wait_for_advisory_wait(pid)
    Timeout.timeout(thread_timeout) do
      loop do
        return if advisory_waiting?(pid)

        Thread.pass
      end
    end
  rescue Timeout::Error
    raise Timeout::Error, "timed out waiting for database session #{pid} to block on the cutoff lock"
  end

  def advisory_waiting?(pid)
    ActiveRecord::Base.connection.select_value(
      ActiveRecord::Base.sanitize_sql_array(
        ["SELECT EXISTS(SELECT 1 FROM pg_locks WHERE pid = ? AND locktype = 'advisory' AND NOT granted)", pid]
      )
    )
  end

  def record_dose(records, service: MedicationAdministration::RecordDose.new)
    service.call(
      source: Schedule.find(records.fetch(:schedule_id)),
      amount_override: nil,
      taken_from_medication_id: records.fetch(:medication_id),
      user: User.find(records.fetch(:user_id))
    )
  end

  def purge_target(test_records)
    Households::Purger.call(
      household: Household.find(test_records.dig(:target, :household_id)),
      actor_account: Account.find(test_records.fetch(:operator_id))
    )
  end

  def cleanup_records
    household_ids = record_household_ids
    User.where(email_address: record_user_emails).delete_all
    HouseholdPurgeRun.where(household_id: household_ids).delete_all
    delete_global_dependencies(household_ids)
    delete_household_inventory(household_ids)
    Household.where(id: household_ids).delete_all
    cleanup_operator
  end

  def cleanup_operator
    operator_id = records.fetch(:operator_id)
    PlatformAdmin.where(account_id: operator_id).delete_all
    Account.where(id: operator_id).delete_all
  end

  def terminate_workers
    workers.each do |worker|
      worker.kill if worker.alive?
      worker.join(1)
    end
  end

  def record_household_ids
    [records.dig(:target, :household_id), records.dig(:unrelated, :household_id)]
  end

  def record_user_emails
    [records.dig(:target, :user_email), records.dig(:unrelated, :user_email)]
  end

  def delete_household_inventory(household_ids)
    (Households::Purger::PURGE_ORDER + %w[household_memberships people]).each do |table_name|
      delete_household_rows(table_name, household_ids)
    end
  end

  def delete_global_dependencies(household_ids)
    membership_ids = HouseholdMembership.where(household_id: household_ids).pluck(:id)
    [ApiSession, ApiAppToken, OauthGrant].each do |model|
      model.where(household_membership_id: membership_ids).delete_all
    end
  end

  def delete_household_rows(table_name, household_ids)
    connection = ActiveRecord::Base.connection
    connection.exec_delete(
      "DELETE FROM #{connection.quote_table_name(table_name)} " \
      "WHERE household_id IN (#{household_ids.map { connection.quote(it) }.join(', ')})"
    )
  end

  def thread_timeout = 10
end

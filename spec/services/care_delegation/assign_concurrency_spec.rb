# frozen_string_literal: true

require 'rails_helper'
require 'timeout'

RSpec.describe CareDelegation::Assign do
  self.use_transactional_tests = false

  before { allow(Audit::Event).to receive(:record!) }

  it 'serializes concurrent assignment at the household boundary' do
    Household.where("name LIKE 'Concurrent Assignment %'").find_each { cleanup_household(it) }
    records = concurrency_records
    outcomes = concurrent_outcomes(records)

    expect(outcomes).to all(be_a(Integer))
    expect(outcomes.uniq.one?).to be(true)
    expect_assignment_counts(records)
  ensure
    cleanup_household(records[:household]) if records
  end

  it 'fails promptly when a worker cannot acquire a database connection' do
    records = concurrency_records
    connection_pool = instance_double(ActiveRecord::ConnectionAdapters::ConnectionPool)
    allow(connection_pool).to receive(:with_connection).and_raise('connection unavailable')

    expect { concurrent_outcomes(records, connection_pool: connection_pool) }
      .to raise_error(RuntimeError, 'connection unavailable')
  ensure
    cleanup_household(records[:household]) if records
  end

  def concurrency_records
    household = Household.create!(name: "Concurrent Assignment #{SecureRandom.hex(4)}")
    carer_account = create_account('concurrent-carer')
    actor_account = create_account('concurrent-actor')
    carer = create(:person, household: household, account: carer_account)
    patient = create(:person, household: household)
    actor = create(:person, household: household, account: actor_account)
    actor_membership = household.household_memberships.create!(
      account: actor_account,
      person: actor,
      role: :owner,
      status: :active
    )
    { household: household, carer: carer, patient: patient, actor_membership: actor_membership }
  end

  def concurrent_outcomes(records, connection_pool: ActiveRecord::Base.connection_pool)
    queues = [Queue.new, Queue.new, Queue.new]
    threads = build_assignment_threads(records, queues, connection_pool)
    run_assignment_threads(threads, queues)
  ensure
    terminate_threads(threads)
  end

  def build_assignment_threads(records, queues, connection_pool)
    ready, start, results = queues
    2.times.map { assignment_thread(records, ready, start, results, connection_pool) }
  end

  def run_assignment_threads(threads, queues)
    ready, start, results = queues
    readiness = 2.times.map { wait_for_queue(ready, 'assignment workers') }
    raise_worker_setup_error(readiness)
    2.times { start << true }
    threads.each { join_thread(it) }
    2.times.map { wait_for_queue(results, 'assignment results') }
  end

  def raise_worker_setup_error(readiness)
    error = readiness.find { it.is_a?(StandardError) }
    raise error if error
  end

  def terminate_threads(threads)
    Array(threads).each do |thread|
      thread.kill if thread.alive?
      thread.join(thread_timeout)
    end
  end

  def assignment_thread(records, ready, start, results, connection_pool)
    Thread.new do
      ready_signaled = false
      begin
        connection_pool.with_connection do
          ready << true
          ready_signaled = true
          wait_for_queue(start, 'assignment start')
          results << assignment_result(records)
        end
      rescue StandardError => e
        ready << e unless ready_signaled
        results << e
      end
    end
  end

  def wait_for_queue(queue, description)
    Timeout.timeout(thread_timeout) { queue.pop }
  rescue Timeout::Error
    raise Timeout::Error, "timed out waiting for #{description}"
  end

  def join_thread(thread)
    return if thread.join(thread_timeout)

    raise Timeout::Error, 'timed out waiting for assignment worker completion'
  end

  def thread_timeout = 10

  def assignment_result(records)
    described_class.new(
      carer: Person.find(records[:carer].id),
      patient: Person.find(records[:patient].id),
      relationship_type: :parent,
      granted_by_membership: HouseholdMembership.find(records[:actor_membership].id)
    ).call.id
  end

  def expect_assignment_counts(records)
    scope = { household: records[:household], carer: records[:carer], patient: records[:patient] }
    expect(CarerRelationship.where(scope).count).to eq(1)
    expect(PersonAccessGrant.where(household: records[:household], person: records[:patient], revoked_at: nil).count)
      .to eq(1)
  end

  def create_account(prefix)
    Account.create!(
      email: "#{prefix}-#{SecureRandom.hex(4)}@example.com",
      password_hash: BCrypt::Password.create('password'),
      status: :verified
    )
  end

  def cleanup_household(household)
    account_ids = household.household_memberships.pluck(:account_id) + household.people.pluck(:account_id).compact
    [PersonAccessGrant, CarerRelationship, HouseholdMembership, LocationMembership, Location, Person].each do |model|
      model.where(household: household).delete_all
    end
    Household.where(id: household.id).delete_all
    Account.where(id: account_ids).delete_all
  end
end

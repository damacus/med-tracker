# frozen_string_literal: true

require 'rails_helper'
require 'timeout'

RSpec.describe PortableData::ImportedPersonAccessGranter do
  self.use_transactional_tests = false

  before { allow(Audit::Event).to receive(:record!) }

  it 'creates one active grant when concurrent imports grant the same person' do
    Household.where("name LIKE 'Concurrent Portable Import %'").find_each { cleanup_household(it) }
    records = concurrency_records
    synchronize_access_changes

    results = concurrent_grants(records)

    expect(results).to all(be_a(Integer))
    expect(results.uniq.one?).to be(true)
    expect(active_grants(records).count).to eq(1)
  ensure
    cleanup_household(records.fetch(:household)) if records
  end

  def concurrency_records
    household = Household.create!(name: "Concurrent Portable Import #{SecureRandom.hex(4)}")
    account = Account.create!(email: "portable-concurrency-#{SecureRandom.hex(4)}@example.test", status: :verified)
    membership = household.household_memberships.create!(account:, role: :owner, status: :active)
    person = Person.create!(
      household:, name: 'Concurrent Import Person', date_of_birth: 30.years.ago,
      person_type: :adult, has_capacity: true
    )
    { household:, account:, membership:, person: }
  end

  def synchronize_access_changes
    gate = TimedGate.new(parties: 2)
    original = Households::AccessChange.method(:for)
    allow(Households::AccessChange).to receive(:for) do |membership|
      gate.arrive
      original.call(membership)
    end
  end

  def concurrent_grants(records)
    start = Queue.new
    threads = 2.times.map { grant_thread(records, start) }
    2.times { start << true }
    threads.map { join_thread(it) }
  ensure
    Array(threads).each(&:kill)
  end

  def grant_thread(records, start)
    Thread.new do
      ActiveRecord::Base.connection_pool.with_connection do
        wait_for(start)
        grant_access(records)
      end
    rescue StandardError => e
      e
    end
  end

  def grant_access(records)
    household = Household.find(records.fetch(:household).id)
    membership = HouseholdMembership.find(records.fetch(:membership).id)
    person = Person.find(records.fetch(:person).id)
    described_class.new(household:, membership:).call(person).id
  end

  def join_thread(thread)
    return thread.value if thread.join(10)

    raise Timeout::Error, 'timed out waiting for portable import grant worker'
  end

  def wait_for(queue)
    Timeout.timeout(10) { queue.pop }
  end

  def active_grants(records)
    PersonAccessGrant.where(
      household: records.fetch(:household),
      household_membership: records.fetch(:membership),
      person: records.fetch(:person),
      revoked_at: nil
    )
  end

  def cleanup_household(household)
    account_ids = household.household_memberships.pluck(:account_id) + household.people.pluck(:account_id).compact
    [PersonAccessGrant, HouseholdMembership, LocationMembership, Location, Person].each do |model|
      model.where(household: household).delete_all
    end
    Household.where(id: household.id).delete_all
    Account.where(id: account_ids).delete_all
  end

  class TimedGate
    def initialize(parties:)
      @parties = parties
      @arrivals = 0
      @mutex = Mutex.new
      @condition = ConditionVariable.new
    end

    def arrive
      mutex.synchronize do
        @arrivals += 1
        arrivals == parties ? condition.broadcast : condition.wait(mutex, 0.25)
      end
    end

    private

    attr_reader :parties, :arrivals, :mutex, :condition
  end
end

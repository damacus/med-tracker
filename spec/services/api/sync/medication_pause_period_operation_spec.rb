require 'rails_helper'
require 'timeout'

RSpec.describe Api::Sync::MedicationPausePeriodOperation do
  self.use_transactional_tests = false

  fixtures :accounts, :households, :schedules

  let(:household) { households(:fixture_household) }
  let(:membership) { accounts(:admin).household_memberships.find_by!(household: household) }
  let(:source) { schedules(:john_paracetamol) }
  let(:operation_attributes) do
    { action: 'create', resource_type: 'medication_pause_period',
      attributes: { source_type: 'schedule', source_id: source.portable_id, reason: 'side_effects' } }
  end

  before do
    FixtureHouseholdSetup.apply!
    reset_source
  end

  after { reset_source }

  it 'reports exactly one concurrent pause as a replay' do
    ready = Queue.new
    start = Queue.new
    workers = 2.times.map { operation_thread(ready, start) }
    2.times { Timeout.timeout(10) { ready.pop } }
    2.times { start << true }
    results = workers.map { join_thread(it) }

    expect(results.map(&:replayed)).to contain_exactly(false, true)
    expect(results.map { it.period.id }.uniq.one?).to be(true)
  ensure
    Array(workers).each(&:kill)
  end

  private

  def operation_thread(ready, start)
    Thread.new do
      ActiveRecord::Base.connection_pool.with_connection { perform_operation(ready, start) }
    rescue StandardError => e
      e
    end
  end

  def perform_operation(ready, start)
    ready << true
    Timeout.timeout(10) { start.pop }
    sync_operation.call(operation: operation_attributes)
  end

  def sync_operation
    operation = described_class.new(authorization: nil, household: household,
                                    membership: HouseholdMembership.find(membership.id))
    stub_authorization(operation)
    operation
  end

  def stub_authorization(operation)
    allow(operation).to receive(:find_source).and_return(source.class.find(source.id))
    allow(operation).to receive(:authorize)
  end

  def join_thread(thread)
    return thread.value if thread.join(10)

    raise Timeout::Error, 'timed out waiting for pause sync worker'
  end

  def reset_source
    periods = MedicationPausePeriod.where(schedule: source)
    PaperTrail::Version.where(item_type: 'MedicationPausePeriod', item_id: periods.select(:id)).delete_all
    periods.delete_all
    source.update!(active: true, retired_at: nil)
  end
end

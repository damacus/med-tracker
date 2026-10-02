require 'rails_helper'
require 'timeout'

RSpec.describe RemoveMedicationStockService do
  self.use_transactional_tests = false
  let(:household) { Household.create!(name: "Stock removal concurrency #{SecureRandom.hex(6)}") }
  let(:medication) do
    Medication.create!(name: 'Concurrency medicine', household: household,
                       location: household.locations.create!(name: 'Test stock'), current_supply: 80)
  end

  after { cleanup_test_household }

  it 'commits only one of two removals exceeding available stock' do
    results = simultaneous_removals([SecureRandom.uuid, SecureRandom.uuid], quantity: '60')
    expect(results.count(&:success?)).to eq(1)
    expect(medication.reload.current_supply).to eq(20)
    expect(described_class.history(medication).count).to eq(1)
  end

  it 'replays simultaneous identical submissions once' do
    submission_id = SecureRandom.uuid
    results = simultaneous_removals([submission_id, submission_id], quantity: '1')
    expect(results).to all(be_success)
    expect(medication.reload.current_supply).to eq(79)
    expect(described_class.history(medication).count).to eq(1)
  end

  def simultaneous_removals(ids, quantity:)
    ready = Queue.new
    start = Queue.new
    threads = ids.map { |submission_id| removal_thread(ready, start, submission_id, quantity) }
    2.times { Timeout.timeout(10) { ready.pop } }
    2.times { start << true }
    threads.map do |thread|
      raise Timeout::Error unless thread.join(10)

      thread.value
    end
  ensure
    Array(threads).each(&:kill)
  end

  def removal_thread(ready, start, submission_id, quantity)
    medication_id = medication.id
    Thread.new do
      ActiveRecord::Base.connection_pool.with_connection do
        record = Medication.find(medication_id)
        ready << true
        Timeout.timeout(10) { start.pop }
        described_class.new.call(medication: record, quantity: quantity, reason: 'lost', submission_id: submission_id)
      end
    end
  end

  def cleanup_test_household
    household_id = household.id
    ActiveRecord::Base.connection.disable_referential_integrity do
      entries = AuditLedgerEntry.where(household_id: household_id)
      AuditExportDelivery.where(audit_ledger_entry_id: entries.select(:id)).delete_all
      [AuditLedgerEntry, AuditChainHead, PaperTrail::Version, ApiChangeEvent, Medication, Location].each do |model|
        model.where(household_id: household_id).delete_all
      end
      Household.where(id: household_id).delete_all
    end
  end
end

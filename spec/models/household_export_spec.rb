# frozen_string_literal: true

require 'rails_helper'

RSpec.describe HouseholdExport do
  def with_runtime_household(household)
    connection.transaction(requires_new: true) do
      connection.execute('SET LOCAL ROLE med_tracker_app')
      connection.execute("SELECT set_config('med_tracker.current_household_id', '#{household.id}', true)")
      yield
      raise ActiveRecord::Rollback
    end
  end

  let(:connection) { ActiveRecord::Base.connection }
  let(:records) do
    account = Account.create!(email: 'lifecycle-rls@example.test', status: :verified)
    household = create(:household)
    other_household = create(:household)
    {
      account: account,
      household: household,
      other_household: other_household,
      export: create_export(household, account),
      other_export: create_export(other_household, account),
      hold: create_hold(household, account, 'RLS preservation'),
      other_hold: create_hold(other_household, account, 'Other RLS preservation')
    }
  end

  it 'isolates export lifecycle and retention hold rows by current household' do
    with_runtime_household(records.fetch(:household)) do
      exports = described_class.where(id: [records.fetch(:export).id, records.fetch(:other_export).id])
      holds = HouseholdRetentionHold.where(id: [records.fetch(:hold).id, records.fetch(:other_hold).id])
      expect(exports.pluck(:id)).to contain_exactly(records.fetch(:export).id)
      expect(holds.pluck(:id)).to contain_exactly(records.fetch(:hold).id)
    end
  end

  it 'rejects cross-household lifecycle writes under the runtime role' do
    with_runtime_household(records.fetch(:household)) do
      expect do
        described_class.create!(
          household: records.fetch(:other_household),
          requested_by_account: records.fetch(:account),
          requested_at: Time.current,
          expires_at: 1.day.from_now
        )
      end.to raise_error(ActiveRecord::StatementInvalid)
    end
  end

  def create_export(household, account)
    described_class.create!(household: household, requested_by_account: account, requested_at: Time.current,
                            expires_at: 1.day.from_now)
  end

  def create_hold(household, account, reason)
    HouseholdRetentionHold.create!(household: household, approved_by_account: account, reason: reason,
                                   review_on: 1.month.from_now.to_date, placed_at: Time.current)
  end
end

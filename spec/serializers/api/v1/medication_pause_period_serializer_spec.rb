require 'rails_helper'

RSpec.describe Api::V1::MedicationPausePeriodSerializer do
  fixtures :accounts, :people, :locations, :medications, :dosages, :schedules

  let(:source) { schedules(:john_paracetamol) }
  let(:period) do
    source.medication_pause_periods.create!(reason: 'reason_not_recorded', legacy_context: true)
  end

  it 'preserves unknown starts and actors in retained legacy history' do
    expect(described_class.new(period).as_json).to include(
      source_id: source.portable_id, reason: 'reason_not_recorded', legacy_context: true,
      started_at: nil, ended_at: nil, recorded_by_membership_id: nil, resumed_by_membership_id: nil,
      recorded_by_name: nil, resumed_by_name: nil
    )
  end

  it 'keeps actor identifiers when their memberships have no linked person' do
    membership = HouseholdMembership.new(id: 123, household: source.household, person: nil)
    period.assign_attributes(recorded_by_membership: membership, resumed_by_membership: membership,
                             ended_at: Time.current)

    expect(described_class.new(period).as_json).to include(
      recorded_by_membership_id: '123', resumed_by_membership_id: '123',
      recorded_by_name: nil, resumed_by_name: nil, ended_at: period.ended_at.iso8601
    )
  end
end

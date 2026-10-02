require 'rails_helper'

RSpec.describe SmartInsights::Context do
  let(:person) { create(:person) }
  let(:source) do
    create(:person_medication, :routine, person: person, dose_cycle: :weekly,
                                         max_daily_doses: 1, created_at: Time.zone.local(2026, 8, 1))
  end

  before { travel_to(Time.zone.local(2026, 9, 9, 12)) }

  def context
    described_class.new(people: [person], start_date: Date.new(2026, 8, 17), end_date: Date.new(2026, 9, 9))
  end

  it 'counts routine cycle expectations and explicit decisions as evidence' do
    create(:medication_take, person_medication: source, taken_at: Time.zone.local(2026, 8, 18, 12))

    expect(context.expected_events).to eq(4)
    expect(context.logged_events).to eq(1)
    expect(context).to be_enough_evidence
    expect(context.prn_takes).to be_empty
  end

  it 'keeps an unfinished weekly cycle out of unexplained misses' do
    source

    expect(context.cycle_summaries.last).to include(unexplained_missed: 0)
  end

  it 'counts an explained decision as evidence without calling it a miss' do
    account = Account.create!(email: 'routine-insight@example.test', status: :verified)
    actor = source.household.household_memberships.create!(account: account, role: :member, status: :active)
    source.medication_dose_occurrences.create!(
      window_starts_on: Date.new(2026, 8, 17), position: 1, outcome: 'not_taken', reason: 'unwell',
      resolved_at: Time.current, resolved_by_membership: actor
    )

    expect(context.logged_events).to eq(1)
    expect(context.cycle_summaries.first).to include(not_taken: 1, unexplained_missed: 0)
  end
end

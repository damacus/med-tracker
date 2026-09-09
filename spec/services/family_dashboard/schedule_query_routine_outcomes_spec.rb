require 'rails_helper'

RSpec.describe FamilyDashboard::ScheduleQuery do
  fixtures :households, :accounts, :people, :locations, :medications, :dosages, :person_medications

  let(:source) do
    create(:person_medication, :routine, person: people(:john), medication: medications(:vitamin_c), max_daily_doses: 1)
  end
  let(:query) { described_class.new([source.person]) }
  let(:actor) do
    account = Account.create!(email: 'routine-dashboard@example.test', status: :verified)
    source.household.household_memberships.create!(account: account, role: :member, status: :active)
  end

  before { travel_to(Time.current.change(hour: 12, min: 0, sec: 0)) }

  def window_start(time = Time.current)
    DoseCycle.new(source.dose_cycle).range_for(time).begin.to_date
  end

  def record_not_taken(date: window_start)
    source.medication_dose_occurrences.create!(
      window_starts_on: date, position: 1, outcome: 'not_taken', reason: 'unwell', note: 'Feeling unwell',
      resolved_at: Time.current, resolved_by_membership: actor
    )
  end

  %w[daily weekly monthly].each do |cycle|
    it "removes a resolved #{cycle} routine dose from outstanding tasks and retains its context" do
      source.update!(dose_cycle: cycle)
      outcome = record_not_taken
      expect(query.call.pluck(:source)).not_to include(source)
      expect(query.today_not_taken_by_person.fetch(source.person)).to include(outcome)
      expect(query.today_takes_by_person.fetch(source.person).map(&:source)).not_to include(source)
    end
  end

  it 'shows remaining untimed positions without treating not-taken as an administration' do
    source.update!(max_daily_doses: 2)
    record_not_taken
    row = query.call.find { |task| task[:source] == source }
    expect(row).to include(daily_dose_count: 0, daily_dose_limit: 2, not_taken_count: 1,
                           scheduled_at: nil, overdue: false)
  end

  it 'restores an outstanding routine task after reopening' do
    record_not_taken.update!(outcome: 'open', reason: nil, note: nil, resolved_at: nil, resolved_by_membership: nil)
    expect(query.call.pluck(:source)).to include(source)
    expect(query.today_not_taken_by_person.fetch(source.person)).to be_empty
  end

  it 'does not apply a previous cycle decision to the current cycle' do
    source.update!(dose_cycle: :weekly)
    record_not_taken(date: window_start(1.week.ago))
    expect(query.call.pluck(:source)).to include(source)
    expect(query.today_not_taken_by_person.fetch(source.person)).to be_empty
  end

  it 'keeps as-needed assignments out of routine outcome presentation' do
    record_not_taken
    source.update!(administration_kind: :as_needed)
    expect(query.call.pluck(:source)).not_to include(source)
    expect(query.today_not_taken_by_person.fetch(source.person)).to be_empty
  end

  it 'includes an early-month take in monthly progress without listing it as taken today' do
    travel_to(Time.zone.local(2026, 7, 31, 12))
    source.update!(dose_cycle: :monthly, max_daily_doses: 3, created_at: Time.current.beginning_of_month)
    source.medication_takes.create!(taken_at: Time.current.beginning_of_month, dose_amount: 500, dose_unit: 'mg')
    record_not_taken
    row = query.call.find { |task| task[:source] == source }
    expect(row).to include(daily_dose_count: 1, daily_dose_limit: 3, not_taken_count: 1)
    expect(query.today_takes_by_person.fetch(source.person).map(&:source)).not_to include(source)
  end
end

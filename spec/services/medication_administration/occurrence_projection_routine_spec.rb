require 'rails_helper'

RSpec.describe MedicationAdministration::OccurrenceProjection do
  fixtures :households, :accounts, :people, :locations, :medications, :dosages, :person_medications

  let(:source) { person_medications(:john_vitamin_d) }
  let(:date) { Date.current }
  let(:membership) do
    source.household.household_memberships.find_or_create_by!(account: accounts(:admin)) do |record|
      record.role = :owner
      record.status = :active
    end
  end

  before { source.update!(created_at: date.beginning_of_month.in_time_zone) }

  def project(first: date, last: date, preloaded: nil)
    described_class.new(source: source, start_date: first, end_date: last,
                        now: date.in_time_zone.change(hour: 12), preloaded: preloaded).call
  end

  def pause(from:, to: nil)
    source.medication_pause_periods.create!(
      reason: 'clinician_advice', started_at: from, ended_at: to,
      recorded_by_membership: membership, resumed_by_membership: to ? membership : nil
    )
  end

  it 'projects distinct stable untimed daily positions without storing them' do
    source.update!(max_daily_doses: 2)
    rows = nil
    expect { rows = project }.not_to change(MedicationDoseOccurrence, :count)
    expect(rows.map { |row| [row.position, row.scheduled_at] }).to eq([[1, nil], [2, nil]])
    expect(rows).to all(be_due)
    expect(project.map(&:key)).to eq(rows.map(&:key))
    expect(described_class.decode(rows.first.key)).to eq(['person_medication', source.portable_id, date.iso8601, 1])
  end

  it 'uses the existing one-dose default when no cycle limit is configured' do
    source.update!(max_daily_doses: nil)
    expect(project.size).to eq(1)
  end

  %w[weekly monthly].each do |cycle|
    it "uses one #{cycle} window when the requested dates are inside that cycle" do
      source.update!(dose_cycle: cycle, created_at: date.beginning_of_year.in_time_zone)
      window = DoseCycle.new(cycle).range_for(date.in_time_zone)
      rows = project(first: window.begin.to_date, last: window.begin.to_date + 2)
      expect(rows.map(&:window_starts_on)).to eq([window.begin.to_date])
      expect(project.map(&:key)).to eq(rows.map(&:key))
    end
  end

  it 'retains the current cycle identity when an assignment was created partway through it' do
    source.update!(dose_cycle: :monthly, created_at: date.in_time_zone)
    expect(project.map(&:window_starts_on)).to eq([date.beginning_of_month])
    expect(project(first: date.prev_month, last: date.prev_month)).to be_empty
  end

  it 'bounds daily expectations by creation and retirement' do
    source.update!(created_at: date.in_time_zone, retired_at: (date + 1).in_time_zone, active: false)
    expect(project(first: date - 1, last: date + 1).map(&:window_starts_on)).to eq([date])
  end

  it 'does not make future windows due' do
    rows = project(first: date + 1, last: date + 1)
    expect(rows.size).to eq(1)
    expect(rows.first).not_to be_due
  end

  it 'does not make a cycle due before the assignment was created' do
    source.update!(dose_cycle: :monthly, created_at: (date + 1).in_time_zone)
    expect(project.size).to eq(1)
    expect(project.first).not_to be_due
  end

  it 'loads a saved cycle outcome when the requested date is inside that cycle' do
    source.update!(dose_cycle: :monthly)
    record = source.medication_dose_occurrences.create!(
      window_starts_on: date.beginning_of_month, position: 1, outcome: 'not_taken',
      resolved_at: Time.current, resolved_by_membership: membership
    )
    expect(project.first).to have_attributes(record: record, outcome: 'not_taken', expected: true)
  end

  it 'excludes as-needed assignments' do
    source.update!(administration_kind: :as_needed)
    expect(project).to be_empty
  end

  it 'keeps partial pauses but excludes a cycle covered by contiguous pauses' do
    source.update!(dose_cycle: :monthly)
    window = DoseCycle.new('monthly').range_for(date.in_time_zone)
    middle = window.begin + 2.days
    pause(from: window.begin, to: middle)
    expect(project.size).to eq(1)
    pause(from: middle)
    source.reload
    expect(project).to be_empty
  end

  it 'retains stored snapshots after the assignment no longer expects the position' do
    record = source.medication_dose_occurrences.create!(
      window_starts_on: date, position: 2, outcome: 'not_taken', reason: 'unwell',
      resolved_at: Time.current, resolved_by_membership: membership
    )
    expect(project.find { |row| row.position == 2 }).to have_attributes(record: record, expected: false)
  end

  it 'allocates unlinked takes within the same cycle and excludes other sources without querying' do
    source.update!(dose_cycle: :monthly, max_daily_doses: 2)
    source.medication_pause_periods.load
    take = MedicationTake.new(id: 101, person_medication: source, taken_at: date.beginning_of_month.in_time_zone)
    other_take = MedicationTake.new(id: 102, person_medication_id: source.id + 1, taken_at: take.taken_at)
    inputs = described_class::Inputs.new(outcomes: [], takes: [other_take, take])
    queries = []
    subscriber = ->(*arguments) { queries << arguments.last[:sql] }
    rows = ActiveSupport::Notifications.subscribed(subscriber, 'sql.active_record') { project(preloaded: inputs) }
    expect(rows.map(&:outcome)).to eq(%w[taken open])
    expect(queries).to be_empty
  end
end

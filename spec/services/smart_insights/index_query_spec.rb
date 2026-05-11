# frozen_string_literal: true

require 'rails_helper'

RSpec.describe SmartInsights::IndexQuery do
  let(:person) { create(:person) }
  let(:people) { Person.where(id: person.id) }
  let(:start_date) { Time.zone.today - 6.days }
  let(:end_date) { Time.zone.today }
  let(:medication) do
    create(:medication, name: 'Vitamin D', current_supply: 500, supply_at_last_restock: 500)
  end

  it 'returns a learning state when there is not enough evidence' do
    create_schedule(start_date: end_date, end_date: end_date)

    result = described_class.new(people: people, start_date: end_date, end_date: end_date).call

    expect(result).to be_learning_state
    expect(result.primary_insight).to be_nil
    expect(result.insights).to be_empty
  end

  it 'ignores evidence and patterns from people outside the caller scope' do
    other_person = create(:person)
    other_schedule = create_schedule(person: other_person, start_date: start_date, end_date: end_date)
    (start_date..end_date).each do |date|
      create_take(schedule: other_schedule, taken_at: date.in_time_zone.change(hour: 8))
    end

    result = described_class.new(people: people, start_date: start_date, end_date: end_date).call

    expect(result).to be_learning_state
    expect(result.insights).to be_empty
  end

  it 'detects an adherence streak only after the evidence threshold is met' do
    schedule = create_schedule(start_date: start_date, end_date: end_date)
    (start_date..end_date).each do |date|
      create_take(schedule: schedule, taken_at: date.in_time_zone.change(hour: 8))
    end

    result = described_class.new(people: people, start_date: start_date, end_date: end_date).call

    expect(result).not_to be_learning_state
    expect(result.insights.map(&:key)).to include(:adherence_streak)
    expect(result.primary_insight.key).to eq(:adherence_streak)
  end

  it 'returns a no-action result when evidence exists but no detector finds a pattern' do
    schedule = create_schedule(start_date: start_date, end_date: end_date)
    [
      [start_date, 8],
      [start_date + 1.day, 10],
      [start_date + 3.days, 12],
      [start_date + 4.days, 14],
      [end_date, 16]
    ].each do |date, hour|
      create_take(schedule: schedule, taken_at: date.in_time_zone.change(hour: hour))
    end

    result = described_class.new(people: people, start_date: start_date, end_date: end_date).call

    expect(result).not_to be_learning_state
    expect(result.primary_insight).to be_nil
    expect(result.insights).to be_empty
  end

  it 'detects missed-dose patterns from expected-versus-actual schedule data' do
    schedule = create_schedule(start_date: start_date, end_date: end_date)
    [start_date, start_date + 1.day, start_date + 4.days, start_date + 5.days, end_date].each do |date|
      create_take(schedule: schedule, taken_at: date.in_time_zone.change(hour: 8))
    end

    result = described_class.new(people: people, start_date: start_date, end_date: end_date).call

    expect(result.insights.map(&:key)).to include(:missed_dose_pattern)
  end

  it 'detects inventory risk using schedule-aware burn rate' do
    low_stock = create(:medication, name: 'Low Stock Med', current_supply: 2, supply_at_last_restock: 20)
    schedule = create_schedule(medication: low_stock, start_date: start_date, end_date: end_date)
    (start_date..end_date).each do |date|
      create_take(schedule: schedule, taken_at: date.in_time_zone.change(hour: 8), decrement_stock: false)
    end

    result = described_class.new(people: people, start_date: start_date, end_date: end_date).call

    expect(result.insights.map(&:key)).to include(:inventory_risk)
  end

  it 'detects timing consistency when logged doses stay near configured times' do
    schedule = create_schedule(start_date: start_date, end_date: end_date)
    (start_date..end_date).each do |date|
      create_take(schedule: schedule, taken_at: date.in_time_zone.change(hour: 8, min: 30))
    end

    result = described_class.new(people: people, start_date: start_date, end_date: end_date).call

    expect(result.insights.map(&:key)).to include(:timing_consistency)
  end

  it 'does not report timing consistency when scheduled occurrences are missed' do
    schedule = create_schedule(
      start_date: start_date,
      end_date: end_date,
      schedule_config: { 'times' => %w[08:00 20:00] },
      max_daily_doses: 2
    )
    (start_date..end_date).each do |date|
      create_take(schedule: schedule, taken_at: date.in_time_zone.change(hour: 8))
    end

    result = described_class.new(people: people, start_date: start_date, end_date: end_date).call

    expect(result.insights.map(&:key)).not_to include(:timing_consistency)
  end

  it 'does not report timing consistency below the on-time threshold' do
    schedule = create_schedule(start_date: start_date, end_date: end_date)
    (start_date..(start_date + 3.days)).each do |date|
      create_take(schedule: schedule, taken_at: date.in_time_zone.change(hour: 8))
    end
    ((start_date + 4.days)..end_date).each do |date|
      create_take(schedule: schedule, taken_at: date.in_time_zone.change(hour: 13))
    end

    result = described_class.new(people: people, start_date: start_date, end_date: end_date).call

    expect(result.insights.map(&:key)).not_to include(:timing_consistency)
  end

  it 'detects as-needed usage from PRN dose logs' do
    schedule = create_schedule(
      start_date: start_date,
      end_date: end_date,
      schedule_config: {},
      schedule_type: :prn,
      max_daily_doses: 4
    )
    (start_date..(start_date + 4.days)).each do |date|
      create_take(schedule: schedule, taken_at: date.in_time_zone.change(hour: 12))
    end

    result = described_class.new(people: people, start_date: start_date, end_date: end_date).call

    expect(result.insights.map(&:key)).to include(:prn_usage)
  end

  it 'does not treat routine person medication takes as PRN usage' do
    schedule = create_schedule(start_date: start_date, end_date: end_date)
    person_medication = create(:person_medication, :routine, person: person, medication: create(:medication))
    (start_date..end_date).each do |date|
      create_take(schedule: schedule, taken_at: date.in_time_zone.change(hour: 8))
      create(:medication_take, :for_person_medication,
             person_medication: person_medication,
             taken_at: date.in_time_zone.change(hour: 12))
    end

    result = described_class.new(people: people, start_date: start_date, end_date: end_date).call

    expect(result.insights.map(&:key)).not_to include(:prn_usage)
  end

  it 'does not report inventory risk at the alert threshold' do
    threshold_stock = create(:medication, name: 'Threshold Stock Med', current_supply: 14, supply_at_last_restock: 20)
    schedule = create_schedule(medication: threshold_stock, start_date: start_date, end_date: end_date)
    (start_date..end_date).each do |date|
      create_take(schedule: schedule, taken_at: date.in_time_zone.change(hour: 8), decrement_stock: false)
    end

    result = described_class.new(people: people, start_date: start_date, end_date: end_date).call

    expect(result.insights.map(&:key)).not_to include(:inventory_risk)
  end

  it 'detects schedule hygiene gaps when active multiple-daily schedules have no configured times' do
    create_schedule(start_date: start_date, end_date: end_date, schedule_config: {})

    result = described_class.new(people: people, start_date: start_date, end_date: end_date).call

    expect(result.insights.map(&:key)).to include(:schedule_hygiene)
  end

  it 'does not report schedule hygiene for configured multiple-daily schedules' do
    schedule = create_schedule(start_date: start_date, end_date: end_date)
    (start_date..end_date).each do |date|
      create_take(schedule: schedule, taken_at: date.in_time_zone.change(hour: 8))
    end

    result = described_class.new(people: people, start_date: start_date, end_date: end_date).call

    expect(result.insights.map(&:key)).not_to include(:schedule_hygiene)
  end

  def create_schedule(**attributes)
    create(
      :schedule,
      {
        person: person,
        medication: medication,
        schedule_type: :multiple_daily,
        schedule_config: { 'times' => ['08:00'] },
        max_daily_doses: 1
      }.merge(attributes)
    )
  end

  def create_take(schedule:, taken_at:, decrement_stock: true)
    take = build(:medication_take, schedule: schedule, person_medication: nil, taken_at: taken_at)
    allow(take).to receive(:decrement_medication_stock) unless decrement_stock
    take.save!
    take
  end
end

# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MedicationDoseDecisionContext do
  fixtures :accounts, :people, :users, :locations, :location_memberships,
           :medications, :dosages, :schedules, :person_medications

  subject(:context) { described_class.new(source: source, taken_at: taken_at) }

  let(:person) { people(:john) }
  let(:taken_at) { Time.zone.local(2026, 7, 2, 9, 0, 0) }
  let(:medication) do
    create(:medication, household: person.household, location: locations(:home), name: 'Decision Context Medicine')
  end
  let(:source) do
    create(
      :schedule,
      person: person,
      medication: medication,
      dosage: nil,
      dose_amount: 500,
      dose_unit: 'mg',
      max_daily_doses: 4,
      min_hours_between_doses: nil,
      start_date: taken_at.to_date - 1.day,
      end_date: taken_at.to_date + 30.days
    )
  end

  before do
    FixtureHouseholdSetup.apply!
    MedicationTake.delete_all
  end

  def record_take_for(source)
    source.medication_takes.create!(
      taken_at: taken_at - 1.hour,
      dose_amount: 500,
      dose_unit: 'mg',
      taken_from_medication: medication,
      taken_from_location: medication.location
    )
  end

  it 'does not block when related sources have no reached restrictions' do
    create(
      :person_medication,
      person: person,
      medication: medication,
      dosage: nil,
      dose_amount: 500,
      dose_unit: 'mg',
      max_daily_doses: 3,
      min_hours_between_doses: nil
    )

    expect(context.blocked?).to be(false)
    expect(context.blocked_reason).to be_nil
    expect(context.audit_payload).to eq(decision_source_count: 2)
  end

  context 'when a related person medication blocks the dose' do
    let!(:related_person_medication) do
      create(
        :person_medication,
        person: person,
        medication: medication,
        dosage: nil,
        dose_amount: 500,
        dose_unit: 'mg',
        max_daily_doses: 1,
        min_hours_between_doses: nil
      )
    end

    before { record_take_for(related_person_medication) }

    it 'reports the related person medication' do
      expect(context.blocked?).to be(true)
      expect(context.blocked_reason).to eq(:overlapping_prescription_restriction)
      expect(context.audit_payload).to eq(
        decision_source_count: 2,
        decision_blocking_source_type: 'person_medication',
        decision_blocking_source_id: related_person_medication.id
      )
    end
  end

  context 'when only inactive or expired schedules have reached restrictions' do
    before do
      expired_schedule = create(
        :schedule,
        :expired,
        person: person,
        medication: medication,
        dosage: nil,
        dose_amount: 500,
        dose_unit: 'mg',
        max_daily_doses: 1,
        min_hours_between_doses: nil,
        start_date: taken_at.to_date - 30.days,
        end_date: taken_at.to_date - 1.day
      )
      inactive_schedule = create(
        :schedule,
        person: person,
        medication: medication,
        dosage: nil,
        dose_amount: 500,
        dose_unit: 'mg',
        max_daily_doses: 1,
        min_hours_between_doses: nil,
        active: false,
        start_date: taken_at.to_date - 1.day,
        end_date: taken_at.to_date + 30.days
      )

      record_take_for(expired_schedule)
      record_take_for(inactive_schedule)
    end

    it 'does not block the dose' do
      expect(context.blocked?).to be(false)
      expect(context.audit_payload).to eq(decision_source_count: 1)
    end
  end
end

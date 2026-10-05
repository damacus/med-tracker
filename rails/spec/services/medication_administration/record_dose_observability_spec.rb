# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MedicationAdministration::RecordDose do
  include ActiveSupport::Testing::TimeHelpers

  fixtures :accounts, :people, :users, :locations, :location_memberships,
           :medications, :dosages, :schedules, :person_medications

  let(:schedule) { schedules(:john_paracetamol) }
  let(:user) { users(:john) }
  let(:events) { [] }

  before do
    FixtureHouseholdSetup.apply!
    MedicationTake.delete_all
    allow(Observability::CanonicalLogger).to receive(:write) { |event| events << event.to_h }
  end

  it 'records one attempt, provisional persistence, and committed outcome' do
    result = record_dose

    aggregate_failures do
      expect(result.success).to be(true)
      expect(event_names).to include(
        'medication_take.attempted',
        'medication_take.persisted',
        'medication_take.committed'
      )
      expect(event_names.count('medication_take.attempted')).to eq(1)
      expect(events_for('medication_take.attempted').sole['event.outcome']).to eq('unknown')
      expect(events_for('medication_take.persisted').sole['medtracker.reason']).to eq('provisional')
      expect(events_for('medication_take.committed').sole['event.outcome']).to eq('success')
    end
  end

  it 'keeps attempt and provisional outcomes but replaces commit with rollback' do
    expect do
      ActiveRecord::Base.transaction do
        expect(record_dose.success).to be(true)
        raise ActiveRecord::Rollback
      end
    end.not_to change(MedicationTake, :count)

    expect(event_names).to include(
      'medication_take.attempted',
      'medication_take.persisted',
      'medication_take.rolled_back'
    )
    expect(event_names).not_to include('medication_take.committed')
  end

  it 'records stable rule blocking and persistence failure outcomes' do
    schedule.medication.update!(current_supply: 0)
    expect(record_dose.error).to eq(:out_of_stock)
    expect(events_for('medication_take.blocked').sole['medtracker.reason']).to eq('out_of_stock')

    events.clear
    schedule.medication.update!(current_supply: 30)
    allow(schedule).to receive(:effective_dose_unit).and_return(nil)
    expect(record_dose.error).to eq(:create_failed)
    expect(events_for('medication_take.failed').sole['medtracker.reason']).to eq('persistence_failed')
  end

  it 'records unavailable household and unexpected failures without hiding the error' do
    allow(schedule).to receive(:reload).and_raise(ActiveRecord::RecordNotFound)

    expect(record_dose.error).to eq(:household_unavailable)
    expect(events_for('medication_take.failed').sole['medtracker.reason']).to eq('household_unavailable')

    events.clear
    allow(schedule).to receive(:reload).and_raise(RuntimeError, 'private failure text')

    expect { record_dose }.to raise_error(RuntimeError, 'private failure text')
    expect(events_for('medication_take.failed').sole).to include(
      'medtracker.reason' => 'unexpected_failure',
      'error.type' => 'RuntimeError'
    )
    expect(events.to_json).not_to include('private failure text')
  end

  it 'keeps one workflow and rotates the attempt identifier across retries' do
    parent = Observability::CorrelationContext.start
    Current.observability_context = parent

    2.times do
      schedule.medication.update!(current_supply: 0)
      record_dose
    end

    attempts = events_for('medication_take.attempted')
    expect(attempts.pluck('medtracker.workflow.id').uniq).to contain_exactly(parent.workflow_id)
    expect(attempts.pluck('medtracker.attempt.id').uniq.size).to eq(2)
    expect(attempts.pluck('event.id').uniq.size).to eq(2)
  ensure
    Current.observability_context = nil
  end

  def record_dose
    MedicationAdministration::RecordDose.new.call(
      source: schedule,
      amount_override: nil,
      taken_from_medication_id: nil,
      user:
    )
  end

  def event_names
    events.pluck('event.name')
  end

  def events_for(name)
    events.select { |event| event['event.name'] == name }
  end
end

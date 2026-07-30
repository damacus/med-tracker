# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Fail-open observability for medication operations' do
  include ActiveSupport::Testing::TimeHelpers

  fixtures :accounts, :people, :users, :locations, :location_memberships,
           :medications, :dosages, :schedules, :person_medications

  let(:schedule) { schedules(:john_paracetamol) }
  let(:user) { users(:john) }

  before do
    FixtureHouseholdSetup.apply!
    MedicationTake.delete_all
  end

  it 'commits a real dose when the canonical logger fails' do
    allow(Observability::CanonicalLogger).to receive(:write).and_raise(IOError, 'stdout unavailable')
    allow(Observability::EmergencyDiagnostic).to receive(:write)

    result = nil
    travel_to Time.current.end_of_day - 1.minute do
      expect do
        result = record_dose
      end.to change(MedicationTake, :count).by(1)
    end

    expect(result.success).to be(true)
    expect(Observability::EmergencyDiagnostic).to have_received(:write).at_least(:once)
  end

  it 'commits a real dose when both logging paths fail' do
    allow(Observability::CanonicalLogger).to receive(:write).and_raise(IOError, 'stdout unavailable')
    allow(Observability::EmergencyDiagnostic).to receive(:write).and_raise(IOError, 'stderr unavailable')

    result = nil
    travel_to Time.current.end_of_day - 1.minute do
      expect do
        result = record_dose
      end.to change(MedicationTake, :count).by(1)
    end

    expect(result.success).to be(true)
    expect(Observability::CanonicalLogger).to have_received(:write).at_least(:once)
  end

  it 'does not change an explicit outer rollback into a commit' do
    allow(Observability::CanonicalLogger).to receive(:write).and_raise(IOError, 'stdout unavailable')
    allow(Observability::EmergencyDiagnostic).to receive(:write).and_raise(IOError, 'stderr unavailable')

    travel_to Time.current.end_of_day - 1.minute do
      expect do
        ActiveRecord::Base.transaction do
          expect(record_dose.success).to be(true)
          raise ActiveRecord::Rollback
        end
      end.not_to change(MedicationTake, :count)
    end
    expect(Observability::CanonicalLogger).to have_received(:write).at_least(:once)
  end

  it 'uses the emergency diagnostic at most once without recursion' do
    allow(Observability::CanonicalLogger).to receive(:write).and_raise(IOError, 'stdout unavailable')
    allow(Observability::EmergencyDiagnostic).to receive(:write).and_raise(IOError, 'stderr unavailable')

    expect do
      Observability::Publisher.emit(
        name: :medication_take_attempted,
        outcome: :unknown,
        severity: :info,
        reason: :requested,
        attributes: { source_category: :schedule }
      )
    end.not_to raise_error
    expect(Observability::EmergencyDiagnostic).to have_received(:write).once
  end

  def record_dose
    MedicationAdministration::RecordDose.new.call(
      source: schedule,
      amount_override: nil,
      taken_from_medication_id: nil,
      user:
    )
  end
end

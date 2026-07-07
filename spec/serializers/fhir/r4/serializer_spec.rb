# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Fhir::R4::Serializer do
  it 'serialises optional Patient and Medication fields only when present' do
    patient = instance_double(Person, portable_id: 'person-portable-id', name: 'Jane Doe', date_of_birth: nil)
    medication = instance_double(
      Medication,
      portable_id: 'medication-portable-id',
      display_name: 'Paracetamol',
      category: nil
    )

    expect(described_class.patient(patient)).not_to have_key(:birthDate)
    expect(described_class.medication(medication).dig(:form, :text)).to be_nil
  end

  it 'serialises stopped medication requests and statements' do
    schedule = stopped_schedule
    person_medication = stopped_person_medication(schedule.person, schedule.medication)

    expect(described_class.medication_request(schedule)[:status]).to eq('stopped')
    expect(described_class.medication_statement(person_medication)[:status]).to eq('stopped')
  end

  it 'omits nil medication administration references and effective time' do
    take = instance_double(
      MedicationTake,
      portable_id: 'take-portable-id',
      schedule: nil,
      person_medication: nil,
      taken_from_medication: nil,
      taken_at: nil
    )

    json = described_class.medication_administration(take)

    expect(json[:subject]).to be_nil
    expect(json[:medicationReference]).to be_nil
    expect(json[:effectiveDateTime]).to be_nil
  end

  it 'builds a bundle by dispatching to the requested resource serializer' do
    patient = instance_double(Person, portable_id: 'person-portable-id', name: 'Jane Doe', date_of_birth: nil)

    json = described_class.bundle([patient], type: :patient)

    expect(json).to include(resourceType: 'Bundle', total: 1)
    expect(json.fetch(:entry).first.fetch(:resource)).to include(resourceType: 'Patient')
  end

  def stopped_schedule
    person = instance_double(Person, portable_id: 'person-portable-id')
    medication = instance_double(Medication, portable_id: 'medication-portable-id')
    instance_double(
      Schedule,
      portable_id: 'schedule-portable-id', active?: false, person: person,
      medication: medication, dose_amount: nil, dose_unit: nil
    )
  end

  def stopped_person_medication(person, medication)
    instance_double(
      PersonMedication,
      portable_id: 'person-medication-portable-id', active?: false,
      person: person, medication: medication, dose_amount: nil, dose_unit: nil
    )
  end
end

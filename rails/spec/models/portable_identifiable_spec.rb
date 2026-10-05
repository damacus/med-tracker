# frozen_string_literal: true

require 'rails_helper'

RSpec.describe PortableIdentifiable do
  let(:uuid_pattern) do
    /\A[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}\z/
  end

  def portable_records
    household = create(:household)
    location = create(:location, household: household)
    person = create(:person, household: household)
    medication = create(:medication, household: household, location: location)
    dosage = create(:dosage, medication: medication, household: household)
    schedule = create(:schedule, person: person, medication: medication, dosage: dosage)
    person_medication = create(:person_medication, person: person, medication: medication, dosage: dosage)
    take = create(:medication_take, :for_schedule, schedule: schedule)
    preference = create(:notification_preference, person: person, household: household)

    [person, location, medication, dosage, schedule, person_medication, take, preference]
  end

  it 'assigns portable IDs to every portable domain record' do
    portable_records.each do |record|
      expect(record.portable_id).to match(uuid_pattern), "#{record.class.name} was missing a portable ID"
    end
  end

  it 'does not allow portable IDs to change after create' do
    person = create(:person)
    person.portable_id = SecureRandom.uuid

    expect(person).not_to be_valid
    expect(person.errors[:portable_id]).to include('cannot be changed')
  end

  it 'requires portable IDs to be unique within a household' do
    portable_id = SecureRandom.uuid
    first_household = create(:household)
    second_household = create(:household)
    create(:location, household: first_household, portable_id: portable_id, name: 'Cabinet')

    duplicate = build(:location, household: first_household, portable_id: portable_id, name: 'Travel Bag')
    same_id_elsewhere = build(:location, household: second_household, portable_id: portable_id, name: 'Cabinet')

    expect(duplicate).not_to be_valid
    expect(duplicate.errors[:portable_id]).to include('has already been taken')
    expect(same_id_elsewhere).to be_valid
  end
end

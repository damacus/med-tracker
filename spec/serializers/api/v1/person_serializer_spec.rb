# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Api::V1::PersonSerializer do
  it 'serialises identity plus age, location ids and notification preference id' do
    person = create(:person, name: 'Alex')
    location = create(:location)
    person.locations << location
    preference = create(:notification_preference, person: person)

    json = described_class.new(person).as_json
    expect(json).to include(
      id: person.id, name: 'Alex', email: person.email, person_type: person.person_type,
      has_capacity: person.has_capacity, updated_at: person.updated_at.iso8601,
      age: person.age, notification_preference_id: preference.id
    )
    expect(json[:location_ids]).to include(location.id)
    expect(json[:date_of_birth]).to eq(person.date_of_birth&.iso8601)
  end

  it 'returns location_ids as an array containing the default location' do
    # Person always gets a default location (Home) on creation via callback
    household = Household.create!(name: 'Person Serializer Household')
    person = create(:person, household: household)
    json = described_class.new(person).as_json
    expect(json[:location_ids]).to be_an(Array)
    expect(json[:location_ids]).not_to be_empty
    expect(json[:location_ids]).to eq(person.locations.map(&:id))
  end

  it 'returns nil notification_preference_id when person has no preference' do
    person = create(:person)
    json = described_class.new(person).as_json
    expect(json[:notification_preference_id]).to be_nil
  end

  it 'serialises the email field' do
    person = create(:person, email: 'alex@example.com')
    expect(described_class.new(person).as_json[:email]).to eq('alex@example.com')
  end

  it 'serialises nil date_of_birth as nil' do
    person = build_stubbed(:person, date_of_birth: nil)
    json = described_class.new(person).as_json
    expect(json[:date_of_birth]).to be_nil
  end

  it 'serialises multiple location ids' do
    person = create(:person)
    loc1 = create(:location)
    loc2 = create(:location)
    person.locations << loc1 << loc2
    json = described_class.new(person).as_json
    # person also has default location, so expect loc1 and loc2 to be included
    expect(json[:location_ids]).to include(loc1.id, loc2.id)
  end
end

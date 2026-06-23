# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Household do
  it 'keeps location membership as household-scoped metadata' do
    household = described_class.create!(name: 'Locationless Family', timezone: Time.zone.name)

    person = household.people.create!(
      name: 'No Location',
      date_of_birth: 30.years.ago.to_date,
      person_type: :adult,
      has_capacity: true
    )

    expect(person.locations.sole).to have_attributes(name: 'Home', household: household)
  end

  it 'scopes location name uniqueness to household case-insensitively' do
    household = described_class.create!(name: 'First Location Family', timezone: Time.zone.name)
    other_household = described_class.create!(name: 'Second Location Family', timezone: Time.zone.name)

    household.locations.create!(name: 'Home')
    duplicate = household.locations.build(name: 'home')
    same_name_elsewhere = other_household.locations.build(name: 'Home')

    expect(duplicate).not_to be_valid
    expect(duplicate.errors[:name]).to include('has already been taken')
    expect(same_name_elsewhere).to be_valid
  end
end

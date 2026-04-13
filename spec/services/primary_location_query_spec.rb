# frozen_string_literal: true

require 'rails_helper'

RSpec.describe PrimaryLocationQuery do
  describe '#call' do
    it 'returns nil when person is nil' do
      expect(described_class.new(person: nil).call).to be_nil
    end

    it 'returns nil when the person has no location memberships' do
      person = create(:person)
      person.location_memberships.delete_all

      expect(described_class.new(person: person).call).to be_nil
    end

    it 'returns the location from the earliest membership by id' do
      person = create(:person)
      person.location_memberships.delete_all
      first_location = create(:location, name: 'First')
      second_location = create(:location, name: 'Second')

      create(:location_membership, person: person, location: first_location)
      create(:location_membership, person: person, location: second_location)

      expect(described_class.new(person: person).call).to eq(first_location)
    end
  end
end

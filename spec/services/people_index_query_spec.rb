# frozen_string_literal: true

require 'rails_helper'

RSpec.describe PeopleIndexQuery do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :schedules, :person_medications

  describe '#call' do
    it 'returns only people from the passed scope with the expected preloads' do
      result = described_class.new(scope: Person.where(id: [people(:john).id])).call

      expect(result).to contain_exactly(people(:john))
      expect(result.first.association(:user)).to be_loaded
      expect(result.first.association(:schedules)).to be_loaded
      expect(result.first.association(:person_medications)).to be_loaded
    end
  end
end

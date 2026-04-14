# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Admin::CarerRelationshipOptionsQuery do
  fixtures :accounts, :people, :users, :locations, :location_memberships

  describe '#call' do
    it 'returns carers excluding minors and patients ordered by name' do
      result = described_class.new(scope: Person.all).call

      expect(result.carers).to include(people(:jane), people(:bob))
      expect(result.carers).not_to include(people(:child_patient))
      expect(result.carers).to include(people(:child_user_person))
      expect(result.patients.map(&:name)).to eq(result.patients.map(&:name).sort)
    end
  end
end

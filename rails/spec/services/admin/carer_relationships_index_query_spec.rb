# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Admin::CarerRelationshipsIndexQuery do
  fixtures :accounts, :people, :users, :carer_relationships

  describe '#call' do
    it 'returns only relationships from the passed scope with expected preloads in newest-first order' do
      older = carer_relationships(:jane_cares_for_child)
      newer = create(:carer_relationship, carer: people(:bob), patient: create(:person), created_at: 1.minute.from_now)

      result = described_class.new(scope: CarerRelationship.where(id: [older.id, newer.id])).call

      expect(result.map(&:id)).to eq([newer.id, older.id])
      expect(result.first.association(:carer)).to be_loaded
      expect(result.first.association(:patient)).to be_loaded
    end
  end
end

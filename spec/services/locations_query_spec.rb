# frozen_string_literal: true

require 'rails_helper'

RSpec.describe LocationsQuery do
  fixtures :locations

  describe '#index' do
    it 'returns only locations from the passed scope with the existing preloads' do
      result = described_class.new(scope: Location.where(id: [locations(:home).id])).index

      expect(result).to contain_exactly(locations(:home))
      expect(result.first.association(:medications)).to be_loaded
      expect(result.first.association(:members)).to be_loaded
      expect(result.first.association(:location_memberships)).to be_loaded
    end
  end

  describe '#find' do
    it 'resolves records within the passed scope with the existing preloads' do
      result = described_class.new(scope: Location.where(id: [locations(:home).id])).find(id: locations(:home).id)

      expect(result).to eq(locations(:home))
      expect(result.association(:medications)).to be_loaded
      expect(result.association(:members)).to be_loaded
      expect(result.association(:location_memberships)).to be_loaded
    end

    it 'raises when the id is outside the passed scope' do
      query = described_class.new(scope: Location.where(id: [locations(:home).id]))

      expect do
        query.find(id: locations(:school).id)
      end.to raise_error(ActiveRecord::RecordNotFound)
    end
  end
end

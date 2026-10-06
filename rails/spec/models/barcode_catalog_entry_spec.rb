# frozen_string_literal: true

require 'rails_helper'

RSpec.describe BarcodeCatalogEntry do
  describe '.normalize_gtin' do
    it 'strips all non-digit characters' do
      expect(described_class.normalize_gtin(' 5-012345_678900 ')).to eq('5012345678900')
    end

    it 'returns an empty string for nil' do
      expect(described_class.normalize_gtin(nil)).to eq('')
    end
  end

  describe 'validations' do
    subject { described_class.new(gtin: '5012345678900', display: 'X', source: 'curated') }

    it { is_expected.to validate_presence_of(:gtin) }
    it { is_expected.to validate_presence_of(:display) }
    it { is_expected.to validate_presence_of(:source) }

    it 'enforces uniqueness of gtin scoped to source' do
      described_class.create!(gtin: '5012345678900', display: 'X', source: 'curated')
      duplicate = described_class.new(gtin: '5012345678900', display: 'Y', source: 'curated')
      expect(duplicate).not_to be_valid
      expect(duplicate.errors[:gtin]).to be_present
    end

    it 'allows the same gtin under a different source' do
      described_class.create!(gtin: '5012345678900', display: 'X', source: 'curated')
      other = described_class.new(gtin: '5012345678900', display: 'Y', source: 'nhs_dmd')
      expect(other).to be_valid
    end
  end
end

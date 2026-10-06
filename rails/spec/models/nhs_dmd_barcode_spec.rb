# frozen_string_literal: true

require 'rails_helper'

RSpec.describe NhsDmdBarcode do
  describe '.normalize_gtin' do
    it 'keeps only digits' do
      expect(described_class.normalize_gtin('  05012-345 ')).to eq('05012345')
    end

    it 'returns an empty string for nil' do
      expect(described_class.normalize_gtin(nil)).to eq('')
    end

    it 'returns an empty string when given a non-string object with no digits' do
      expect(described_class.normalize_gtin(123)).to eq('123')
    end
  end

  describe 'validations' do
    subject { described_class.new(gtin: '5012345678900', code: 'C', display: 'D', system: 'S') }

    it { is_expected.to validate_presence_of(:gtin) }

    it 'enforces uniqueness of gtin' do
      described_class.create!(gtin: '5012345678900', code: 'C', display: 'D', system: 'S')
      duplicate = described_class.new(gtin: '5012345678900', code: 'C2', display: 'D2', system: 'S2')
      expect(duplicate).not_to be_valid
      expect(duplicate.errors[:gtin]).to be_present
    end

    it { is_expected.to validate_presence_of(:code) }
    it { is_expected.to validate_presence_of(:display) }
    it { is_expected.to validate_presence_of(:system) }
  end

  it 'expires the barcode-lookup cache after commit' do
    allow(NhsDmd::BarcodeLookup).to receive(:expire)
    described_class.create!(gtin: '5012345678900', code: 'C', display: 'D', system: 'S')
    expect(NhsDmd::BarcodeLookup).to have_received(:expire).with('5012345678900')
  end
end

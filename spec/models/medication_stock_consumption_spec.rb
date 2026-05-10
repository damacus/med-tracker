# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MedicationStockConsumption do
  describe '.quantity_for' do
    it 'uses the dose amount for volume units' do
      expect(described_class.quantity_for(dose_amount: 2.5, dose_unit: 'ml')).to eq(BigDecimal('2.5'))
    end

    it 'uses the dose amount for countable package units' do
      expect(described_class.quantity_for(dose_amount: 2, dose_unit: 'gummy')).to eq(BigDecimal('2'))
      expect(described_class.quantity_for(dose_amount: 2, dose_unit: 'tablet')).to eq(BigDecimal('2'))
    end

    it 'uses one stock unit for strength-only units' do
      expect(described_class.quantity_for(dose_amount: 500, dose_unit: 'mg')).to eq(BigDecimal('1'))
      expect(described_class.quantity_for(dose_amount: 1000, dose_unit: 'IU')).to eq(BigDecimal('1'))
    end

    it 'returns zero when dose amount is blank' do
      expect(described_class.quantity_for(dose_amount: nil, dose_unit: 'gummy')).to eq(BigDecimal('0'))
    end
  end
end

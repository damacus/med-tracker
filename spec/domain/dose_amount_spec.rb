# frozen_string_literal: true

require 'rails_helper'

RSpec.describe DoseAmount do
  describe '#label' do
    it 'formats integer amounts without decimal' do
      expect(described_class.new(5, 'mg').label).to eq('5 mg')
    end

    it 'formats float amounts stripping trailing .0' do
      expect(described_class.new(5.0, 'mg').label).to eq('5 mg')
    end

    it 'formats decimal integer amounts without decimal' do
      expect(described_class.new(BigDecimal('1.0'), 'tablet').label).to eq('1 tablet')
    end

    it 'preserves meaningful decimal places' do
      expect(described_class.new(2.5, 'ml').label).to eq('2.5 ml')
    end

    it 'returns nil when amount is blank' do
      expect(described_class.new(nil, 'mg').label).to be_nil
    end

    it 'returns nil when unit is blank' do
      expect(described_class.new(5, nil).label).to be_nil
    end

    it 'returns nil when both are blank' do
      expect(described_class.new(nil, nil).label).to be_nil
    end

    it 'formats string amounts by coercing to decimal' do
      expect(described_class.new('10', 'mg').label).to eq('10 mg')
    end

    it 'keeps countable units singular for one dose' do
      expect(described_class.new(1, 'tablet').label).to eq('1 tablet')
    end

    it 'pluralizes countable units for multiple doses' do
      expect(described_class.new(2, 'tablet').label).to eq('2 tablets')
    end

    it 'pluralizes gummies for multiple doses' do
      expect(described_class.new(2, 'gummy').label).to eq('2 gummies')
    end

    it 'does not pluralize measurement abbreviations' do
      expect(described_class.new(2, 'mg').label).to eq('2 mg')
    end
  end
end

# frozen_string_literal: true

require 'rails_helper'

RSpec.describe DoseAmount do
  describe '#to_s' do
    it 'formats integer amounts without decimal' do
      expect(described_class.new(5, 'mg').to_s).to eq('5 mg')
    end

    it 'formats float amounts stripping trailing .0' do
      expect(described_class.new(5.0, 'mg').to_s).to eq('5 mg')
    end

    it 'preserves meaningful decimal places' do
      expect(described_class.new(2.5, 'ml').to_s).to eq('2.5 ml')
    end

    it 'returns empty string when amount is blank' do
      expect(described_class.new(nil, 'mg').to_s).to eq('')
    end

    it 'returns empty string when unit is blank' do
      expect(described_class.new(5, nil).to_s).to eq('')
    end

    it 'returns empty string when both are blank' do
      expect(described_class.new(nil, nil).to_s).to eq('')
    end

    it 'formats string amounts by coercing to float' do
      expect(described_class.new('10', 'mg').to_s).to eq('10 mg')
    end
  end
end

# frozen_string_literal: true

require 'rails_helper'

RSpec.describe NhsDmd::StrengthFilter do
  describe '.normalize' do
    it 'normalizes spacing, unit names, and equivalent mass units' do
      expect(described_class.normalize('500 milligrams')).to eq('500mg')
      expect(described_class.normalize('0.5 g')).to eq('500mg')
      expect(described_class.normalize('500,000 micrograms')).to eq('500mg')
    end

    it 'normalizes concentration strengths' do
      expect(described_class.normalize('250 mg / 5 mL')).to eq('250mg/5ml')
    end

    it 'returns nil for unsupported values' do
      expect(described_class.normalize('strong')).to be_nil
    end
  end

  describe '.filter' do
    let(:tablet) do
      NhsDmd::SearchResult.new(
        code: '1',
        display: 'Paracetamol 500mg tablets',
        system: 'https://dmd.nhs.uk'
      )
    end
    let(:liquid) do
      NhsDmd::SearchResult.new(
        code: '2',
        display: 'Paracetamol 250 mg / 5 mL oral suspension',
        system: 'https://dmd.nhs.uk'
      )
    end
    let(:stronger_tablet) do
      NhsDmd::SearchResult.new(
        code: '3',
        display: 'Paracetamol 5000mg tablets',
        system: 'https://dmd.nhs.uk'
      )
    end

    it 'returns all results when strength is blank' do
      expect(described_class.filter([tablet, liquid], nil)).to eq([tablet, liquid])
    end

    it 'returns only exact normalized strength matches' do
      expect(described_class.filter([tablet, liquid, stronger_tablet], '0.5g')).to eq([tablet])
    end

    it 'matches normalized concentration formats' do
      expect(described_class.filter([tablet, liquid], '250mg/5ml')).to eq([liquid])
    end
  end
end

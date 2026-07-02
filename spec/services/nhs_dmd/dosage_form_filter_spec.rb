# frozen_string_literal: true

require 'rails_helper'

RSpec.describe NhsDmd::DosageFormFilter do
  describe '.normalize' do
    it 'normalizes common form aliases' do
      expect(described_class.normalize('Tablets')).to eq('tablet')
      expect(described_class.normalize('oral solution')).to eq('liquid')
      expect(described_class.normalize('inhalation powder')).to eq('inhaler')
    end

    it 'returns nil for unsupported forms' do
      expect(described_class.normalize('unknown')).to be_nil
    end
  end

  describe '.filter' do
    let(:tablet) do
      NhsDmd::SearchResult.new(
        code: '1',
        display: 'Aspirin 300mg tablets',
        system: 'https://dmd.nhs.uk',
        package_unit: 'tablet'
      )
    end
    let(:liquid) do
      NhsDmd::SearchResult.new(
        code: '2',
        display: 'Amoxicillin 250mg/5ml oral suspension',
        system: 'https://dmd.nhs.uk',
        package_unit: 'ml'
      )
    end

    it 'returns all results when no form is selected' do
      expect(described_class.filter([tablet, liquid], nil)).to eq([tablet, liquid])
    end

    it 'returns only results matching the normalized form' do
      expect(described_class.filter([tablet, liquid], 'liquid')).to eq([liquid])
    end
  end
end

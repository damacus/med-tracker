# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MedicationFriendlyName do
  describe '.derive' do
    it 'returns the leading non-dosage words as the friendly name' do
      result = described_class.derive(name: 'Paracetamol 500mg tablets', code: '12345')
      expect(result).to eq('Paracetamol')
    end

    it 'keeps multiple leading words joined by single spaces' do
      result = described_class.derive(name: 'Movicol Paediatric Plain 13.8g sachets', code: '12345')
      expect(result).to eq('Movicol Paediatric Plain')
    end

    it 'strips parenthetical manufacturer text before deriving' do
      result = described_class.derive(name: 'Movicol (Norgine) Plain 13.8g', code: '12345')
      expect(result).to eq('Movicol Plain')
    end

    it 'stops at the first dosage token rather than collecting all friendly words' do
      # take_while (not select): a friendly word AFTER a dosage token is excluded.
      result = described_class.derive(name: 'Vitamin 500mg Plain', code: '12345')
      expect(result).to eq('Vitamin')
    end

    it 'trims a trailing comma left on the last kept word' do
      result = described_class.derive(name: 'Ibuprofen, 200mg tablets', code: '12345')
      expect(result).to eq('Ibuprofen')
    end

    it 'filters stop words case-insensitively' do
      # Without downcase, "TABLETS" would not match the lower-case stop list.
      result = described_class.derive(name: 'Paracetamol TABLETS', code: '12345')
      expect(result).to eq('Paracetamol')
    end

    it 'filters stop words even when punctuation is attached' do
      # Without delete('.,'), "tablets." would not match the stop list.
      result = described_class.derive(name: 'Paracetamol tablets. extra', code: '12345')
      expect(result).to eq('Paracetamol')
    end

    it 'treats a plain trailing number as a dosage token' do
      # Exercises the bare-number regex branch (no unit suffix).
      result = described_class.derive(name: 'Paracetamol 500 Plain', code: '12345')
      expect(result).to eq('Paracetamol')
    end

    it 'treats unit-suffixed numbers as dosage tokens' do
      result = described_class.derive(name: 'Amoxicillin 250mg capsules', code: '12345')
      expect(result).to eq('Amoxicillin')
    end

    it 'treats decimal unit-suffixed numbers as dosage tokens' do
      result = described_class.derive(name: 'Movicol 6.9g powder', code: '12345')
      expect(result).to eq('Movicol')
    end

    it 'treats percentage tokens as dosage tokens' do
      result = described_class.derive(name: 'Hydrocortisone 1% cream', code: '12345')
      expect(result).to eq('Hydrocortisone')
    end

    it 'returns nil when the code is blank' do
      expect(described_class.derive(name: 'Paracetamol 500mg tablets', code: '')).to be_nil
    end

    it 'returns nil when the name is blank' do
      expect(described_class.derive(name: '', code: '12345')).to be_nil
    end

    it 'returns the friendly name when both name and code are present' do
      # Guards the code.blank? || name.blank? short-circuit (both present → not nil).
      expect(described_class.derive(name: 'Paracetamol 500mg', code: '12345')).to eq('Paracetamol')
    end

    it 'returns nil when no leading word survives the stop-word filter' do
      expect(described_class.derive(name: 'tablets 500mg', code: '12345')).to be_nil
    end

    it 'returns nil when the friendly name would equal the original name' do
      expect(described_class.derive(name: 'Paracetamol', code: '12345')).to be_nil
    end
  end
end

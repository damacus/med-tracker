# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MedicationDosageOption do
  describe 'timing defaults' do
    it 'requires max doses, min hours apart, and dose cycle' do
      dosage_option = build(
        :dosage,
        default_max_daily_doses: nil,
        default_min_hours_between_doses: nil,
        default_dose_cycle: nil
      )

      expect(dosage_option).not_to be_valid
      expect(dosage_option.errors[:default_max_daily_doses]).to include("can't be blank")
      expect(dosage_option.errors[:default_min_hours_between_doses]).to include("can't be blank")
      expect(dosage_option.errors[:default_dose_cycle]).to include("can't be blank")
    end
  end

  describe 'unit alignment' do
    it 'follows the medication main unit' do
      medication = build(:medication, dosage_unit: 'ml')
      dosage_option = build(:dosage, medication: medication, unit: 'mg')

      dosage_option.valid?

      expect(dosage_option.unit).to eq('ml')
    end
  end
end

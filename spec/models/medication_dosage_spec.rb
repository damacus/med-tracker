# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MedicationDosage do
  subject(:dosage) do
    described_class.new(
      amount: 500,
      unit: 'mg',
      frequency: 'Twice daily',
      description: 'Take with food',
      default_for_adults: true,
      default_for_children: false,
      default_max_daily_doses: 4,
      default_min_hours_between_doses: 6,
      default_dose_cycle: 'daily'
    )
  end

  describe 'value semantics' do
    it 'compares equality by content' do
      same_dosage = described_class.new(
        amount: 500,
        unit: 'mg',
        frequency: 'Twice daily',
        description: 'Take with food',
        default_for_adults: true,
        default_for_children: false,
        default_max_daily_doses: 4,
        default_min_hours_between_doses: 6,
        default_dose_cycle: 'daily'
      )

      expect(dosage).to eq(same_dosage)
    end

    it 'preserves the dosage content attributes' do
      expect(dosage).to have_attributes(
        amount: 500,
        unit: 'mg',
        frequency: 'Twice daily',
        description: 'Take with food',
        default_for_adults: true,
        default_for_children: false,
        default_max_daily_doses: 4,
        default_min_hours_between_doses: 6,
        default_dose_cycle: 'daily'
      )
    end

    it 'formats the amount and unit using DoseAmount semantics' do
      expect(dosage.to_s).to eq('500 mg')
    end

    it 'derives a selection key from the dose content' do
      expect(dosage.selection_key).to eq('500|mg')
    end
  end
end

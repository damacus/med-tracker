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
    it 'allows a dose option to keep its own unit' do
      medication = build(:medication, dosage_unit: 'tablet')
      dosage_option = build(:dosage, medication: medication, unit: 'capsule')

      dosage_option.valid?

      expect(dosage_option.unit).to eq('capsule')
    end
  end

  describe 'inventory fields' do
    it 'allows nil inventory values for legacy dose options' do
      dosage_option = build(:dosage, current_supply: nil, reorder_threshold: nil)

      expect(dosage_option).to be_valid
    end

    it 'validates non-negative tracked inventory values' do
      dosage_option = build(:dosage, current_supply: -1, reorder_threshold: -1)

      expect(dosage_option).not_to be_valid
      expect(dosage_option.errors[:current_supply]).to include('must be greater than or equal to 0')
      expect(dosage_option.errors[:reorder_threshold]).to include('must be greater than or equal to 0')
    end

    it 'can suppress one inventory sync for explicit locked inventory updates' do
      dosage_option = create(:dosage, current_supply: 5, reorder_threshold: 1)

      dosage_option.with_inventory_sync_suppressed do
        dosage_option.current_supply = 4

        expect(dosage_option.send(:tracked_inventory_change?)).to be(false)
      end

      dosage_option.send(:reset_inventory_sync_suppression)

      expect(dosage_option.send(:tracked_inventory_change?)).to be(true)
    end

    it 'resets suppressed inventory sync when the wrapped update fails before commit' do
      dosage_option = create(:dosage, current_supply: 5, reorder_threshold: 1)

      expect do
        dosage_option.with_inventory_sync_suppressed do
          raise ActiveRecord::RecordInvalid, dosage_option
        end
      end.to raise_error(ActiveRecord::RecordInvalid)

      dosage_option.current_supply = 4

      expect(dosage_option.send(:tracked_inventory_change?)).to be(true)
    end
  end
end

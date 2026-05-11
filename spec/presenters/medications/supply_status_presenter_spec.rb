# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Medications::SupplyStatusPresenter do
  describe '#status_variant' do
    it 'prioritizes reorder ordered status over stock state' do
      medication = create(:medication,
                          current_supply: 5,
                          reorder_threshold: 10,
                          reorder_status: :ordered)

      presenter = described_class.new(medication:)

      expect(presenter.status_variant).to eq(:default)
      expect(presenter.status_label).to eq('Ordered')
    end

    it 'prioritizes reorder received status over stock state' do
      medication = create(:medication,
                          current_supply: 5,
                          reorder_threshold: 10,
                          reorder_status: :received)

      presenter = described_class.new(medication:)

      expect(presenter.status_variant).to eq(:success)
      expect(presenter.status_label).to eq('Received')
    end

    it 'returns warning for low stock medications' do
      medication = create(:medication, current_supply: 5, reorder_threshold: 10)

      presenter = described_class.new(medication:)

      expect(presenter.status_variant).to eq(:warning)
      expect(presenter.status_label).to eq('Low Stock Alert')
    end

    it 'returns destructive for out of stock medications' do
      medication = create(:medication, current_supply: 0, reorder_threshold: 10)

      presenter = described_class.new(medication:)

      expect(presenter.status_variant).to eq(:destructive)
      expect(presenter.status_label).to eq('Out of Stock')
    end

    it 'returns success for in stock medications' do
      medication = create(:medication, current_supply: 50, reorder_threshold: 10)

      presenter = described_class.new(medication:)

      expect(presenter.status_variant).to eq(:success)
      expect(presenter.status_label).to eq('In Stock')
    end
  end

  describe '#stock_count_class' do
    it 'returns error class when stock is low' do
      medication = create(:medication, current_supply: 5, reorder_threshold: 10)
      presenter = described_class.new(medication:)

      expect(presenter.stock_count_class).to eq('text-5xl font-black text-on-error-container')
    end

    it 'returns primary class when stock is normal' do
      medication = create(:medication, current_supply: 50, reorder_threshold: 10)
      presenter = described_class.new(medication:)

      expect(presenter.stock_count_class).to eq('text-5xl font-black text-primary')
    end
  end

  describe '#supply_bar_class' do
    it 'returns error class when stock is low' do
      medication = create(:medication, current_supply: 5, reorder_threshold: 10)
      presenter = described_class.new(medication:)

      expect(presenter.supply_bar_class).to eq('bg-error')
    end

    it 'returns primary class when stock is normal' do
      medication = create(:medication, current_supply: 50, reorder_threshold: 10)
      presenter = described_class.new(medication:)

      expect(presenter.supply_bar_class).to eq('bg-primary')
    end
  end

  describe '#list_supply_bar_class' do
    it 'returns destructive class when stock is low' do
      medication = create(:medication, current_supply: 5, reorder_threshold: 10)
      presenter = described_class.new(medication:)

      expect(presenter.list_supply_bar_class).to eq('bg-destructive')
    end

    it 'returns primary class when stock is normal' do
      medication = create(:medication, current_supply: 50, reorder_threshold: 10)
      presenter = described_class.new(medication:)

      expect(presenter.list_supply_bar_class).to eq('bg-primary')
    end

    it 'returns destructive class when scheduled medication has less than five days left' do
      medication = create(:medication, current_supply: 4, reorder_threshold: 0)
      create(:schedule, medication: medication, dose_amount: 500, dose_unit: 'mg', max_daily_doses: 1,
                        frequency: 'Once daily')
      presenter = described_class.new(medication:)

      expect(presenter.list_supply_bar_class).to eq('bg-destructive')
    end

    it 'returns primary class when scheduled medication has five days left' do
      medication = create(:medication, current_supply: 5, reorder_threshold: 10)
      create(:schedule, medication: medication, dose_amount: 500, dose_unit: 'mg', max_daily_doses: 1,
                        frequency: 'Once daily')
      presenter = described_class.new(medication:)

      expect(presenter.list_supply_bar_class).to eq('bg-primary')
    end

    it 'returns destructive class when as-needed medication has less than ten doses left' do
      medication = create(:medication, current_supply: 18, reorder_threshold: 0)
      create(:person_medication, :as_needed, medication: medication, dose_amount: 2, dose_unit: 'tablet')
      presenter = described_class.new(medication:)

      expect(presenter.list_supply_bar_class).to eq('bg-destructive')
    end

    it 'returns primary class when as-needed medication has ten doses left' do
      medication = create(:medication, current_supply: 20, reorder_threshold: 10)
      create(:person_medication, :as_needed, medication: medication, dose_amount: 2, dose_unit: 'tablet')
      presenter = described_class.new(medication:)

      expect(presenter.list_supply_bar_class).to eq('bg-primary')
    end
  end

  describe '#list_inventory_text_class' do
    it 'returns destructive class for red inventory levels' do
      medication = create(:medication, current_supply: 4, reorder_threshold: 0)
      create(:schedule, medication: medication, dose_amount: 500, dose_unit: 'mg', max_daily_doses: 1,
                        frequency: 'Once daily')
      presenter = described_class.new(medication:)

      expect(presenter.list_inventory_text_class).to eq('text-destructive')
    end

    it 'returns primary class for blue inventory levels' do
      medication = create(:medication, current_supply: 5, reorder_threshold: 10)
      create(:schedule, medication: medication, dose_amount: 500, dose_unit: 'mg', max_daily_doses: 1,
                        frequency: 'Once daily')
      presenter = described_class.new(medication:)

      expect(presenter.list_inventory_text_class).to eq('text-primary')
    end
  end

  describe '#remaining_units_label' do
    it 'returns singular label for 1 unit' do
      medication = create(:medication, current_supply: 1, reorder_threshold: 10)
      presenter = described_class.new(medication:)

      expect(presenter.remaining_units_label).to eq('unit remaining')
    end

    it 'returns plural label for multiple units' do
      medication = create(:medication, current_supply: 5, reorder_threshold: 10)
      presenter = described_class.new(medication:)

      expect(presenter.remaining_units_label).to eq('units remaining')
    end

    it 'returns plural label for 0 units' do
      medication = create(:medication, current_supply: 0, reorder_threshold: 10)
      presenter = described_class.new(medication:)

      expect(presenter.remaining_units_label).to eq('units remaining')
    end

    it 'returns ml label for volume stock' do
      medication = create(:medication, dosage_unit: 'ml', current_supply: 97.5, reorder_threshold: 10)
      presenter = described_class.new(medication:)

      expect(presenter.remaining_units_label).to eq('ml remaining')
    end
  end

  describe '#inventory_units_label' do
    it 'returns singular label for 1 unit' do
      medication = create(:medication, current_supply: 1, reorder_threshold: 10)
      presenter = described_class.new(medication:)

      expect(presenter.inventory_units_label).to eq('1 unit')
    end

    it 'returns plural label for multiple units' do
      medication = create(:medication, current_supply: 5, reorder_threshold: 10)
      presenter = described_class.new(medication:)

      expect(presenter.inventory_units_label).to eq('5 units')
    end

    it 'returns decimal ml label for volume stock' do
      medication = create(:medication, dosage_unit: 'ml', current_supply: 97.5, reorder_threshold: 10)
      presenter = described_class.new(medication:)

      expect(presenter.inventory_units_label).to eq('97.5 ml')
    end
  end

  describe '#forecast_items' do
    it 'builds the low-stock and empty forecasts' do
      medication = create(:medication, current_supply: 50, reorder_threshold: 10)
      create(
        :schedule,
        medication: medication,
        dose_amount: 500,
        dose_unit: 'mg',
        max_daily_doses: 10,
        dose_cycle: :daily
      )

      presenter = described_class.new(medication:)

      expected_items = [
        { message: 'Supply will be low in 4 days', variant: :warning },
        { message: 'Supply will be empty in 5 days', variant: :destructive }
      ]

      expect(presenter.forecast_items).to eq(expected_items)
    end

    it 'returns an empty list when no forecast is available' do
      presenter = described_class.new(medication: create(:medication, current_supply: 50, reorder_threshold: 10))

      expect(presenter.forecast_items).to eq([])
    end
  end

  describe '#reorder_status_badge?' do
    it 'only shows reorder status when the medication is low stock and reordered' do
      low_stock = create(:medication, current_supply: 5, reorder_threshold: 10, reorder_status: :ordered)
      in_stock = create(:medication, current_supply: 50, reorder_threshold: 10, reorder_status: :ordered)

      expect(described_class.new(medication: low_stock).reorder_status_badge?).to be(true)
      expect(described_class.new(medication: in_stock).reorder_status_badge?).to be(false)
    end
  end

  describe '#reorder_status_variant' do
    it 'returns default for ordered status' do
      medication = create(:medication, current_supply: 5, reorder_threshold: 10, reorder_status: :ordered)
      presenter = described_class.new(medication:)

      expect(presenter.reorder_status_variant).to eq(:default)
    end

    it 'returns success for received status' do
      medication = create(:medication, current_supply: 5, reorder_threshold: 10, reorder_status: :received)
      presenter = described_class.new(medication:)

      expect(presenter.reorder_status_variant).to eq(:success)
    end

    it 'returns outline when there is no status' do
      medication = create(:medication, current_supply: 5, reorder_threshold: 10, reorder_status: nil)
      presenter = described_class.new(medication:)

      expect(presenter.reorder_status_variant).to eq(:outline)
    end
  end

  describe '#reorder_status_label' do
    it 'returns label for ordered status' do
      medication = create(:medication, current_supply: 5, reorder_threshold: 10, reorder_status: :ordered)
      presenter = described_class.new(medication:)

      expect(presenter.reorder_status_label).to eq('Ordered')
    end

    it 'returns label for received status' do
      medication = create(:medication, current_supply: 5, reorder_threshold: 10, reorder_status: :received)
      presenter = described_class.new(medication:)

      expect(presenter.reorder_status_label).to eq('Received')
    end
  end

  describe '#reorder_status_timestamp' do
    it 'returns reordered_at for received status' do
      timestamp = Time.zone.parse('2024-01-01 12:00:00')
      medication = create(:medication,
                          current_supply: 5,
                          reorder_threshold: 10,
                          reorder_status: :received,
                          reordered_at: timestamp)
      presenter = described_class.new(medication:)

      expect(presenter.reorder_status_timestamp).to eq(timestamp)
    end

    it 'returns ordered_at for ordered status' do
      timestamp = Time.zone.parse('2024-01-01 12:00:00')
      medication = create(:medication,
                          current_supply: 5,
                          reorder_threshold: 10,
                          reorder_status: :ordered,
                          ordered_at: timestamp)
      presenter = described_class.new(medication:)

      expect(presenter.reorder_status_timestamp).to eq(timestamp)
    end

    it 'returns nil when no status' do
      medication = create(:medication, current_supply: 5, reorder_threshold: 10, reorder_status: nil)
      presenter = described_class.new(medication:)

      expect(presenter.reorder_status_timestamp).to be_nil
    end
  end
end

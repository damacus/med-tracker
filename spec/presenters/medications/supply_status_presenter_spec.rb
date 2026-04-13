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

    it 'returns warning for low stock medications' do
      medication = create(:medication, current_supply: 5, reorder_threshold: 10)

      presenter = described_class.new(medication:)

      expect(presenter.status_variant).to eq(:warning)
      expect(presenter.status_label).to eq('⚠️ Low Stock Alert')
    end

    it 'returns success for in stock medications' do
      medication = create(:medication, current_supply: 50, reorder_threshold: 10)

      presenter = described_class.new(medication:)

      expect(presenter.status_variant).to eq(:success)
      expect(presenter.status_label).to eq('In Stock')
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
end

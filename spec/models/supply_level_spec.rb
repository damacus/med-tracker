# frozen_string_literal: true

require 'rails_helper'

RSpec.describe SupplyLevel do
  describe '#current' do
    it 'returns zero when current supply is nil' do
      supply_level = described_class.new(current: nil, reorder_threshold: 10, last_restock: nil)

      expect(supply_level.current).to eq(0)
    end
  end

  describe '#percentage' do
    it 'uses supply at last restock as the denominator when present' do
      supply_level = described_class.new(current: 40, reorder_threshold: 10, last_restock: 80)

      expect(supply_level.percentage).to eq(50)
    end

    it 'falls back to reorder threshold when last restock is nil' do
      supply_level = described_class.new(current: 40, reorder_threshold: 10, last_restock: nil)

      expect(supply_level.percentage).to eq(100)
    end

    it 'caps percentage at 100' do
      supply_level = described_class.new(current: 100, reorder_threshold: 10, last_restock: 80)

      expect(supply_level.percentage).to eq(100)
    end
  end

  describe '#low_stock?' do
    it 'returns true when current supply meets the reorder threshold' do
      supply_level = described_class.new(current: 10, reorder_threshold: 10, last_restock: 50)

      expect(supply_level).to be_low_stock
    end
  end

  describe '#out_of_stock?' do
    it 'returns false when current supply is nil' do
      supply_level = described_class.new(current: nil, reorder_threshold: 10, last_restock: nil)

      expect(supply_level).not_to be_out_of_stock
    end
  end

  describe '#status' do
    it 'returns out_of_stock before low_stock' do
      supply_level = described_class.new(current: 0, reorder_threshold: 10, last_restock: 50)

      expect(supply_level.status).to eq(:out_of_stock)
    end

    it 'returns low_stock at the reorder threshold' do
      supply_level = described_class.new(current: 10, reorder_threshold: 10, last_restock: 50)

      expect(supply_level.status).to eq(:low_stock)
    end

    it 'returns in_stock above the reorder threshold' do
      supply_level = described_class.new(current: 11, reorder_threshold: 10, last_restock: 50)

      expect(supply_level.status).to eq(:in_stock)
    end
  end

  describe '#days_until_low_stock' do
    it 'returns nil when daily consumption is not positive' do
      supply_level = described_class.new(current: 50, reorder_threshold: 10, last_restock: 80)

      expect(supply_level.days_until_low_stock(daily_consumption: 0)).to be_nil
    end

    it 'returns zero when already low stock' do
      supply_level = described_class.new(current: 10, reorder_threshold: 10, last_restock: 50)

      expect(supply_level.days_until_low_stock(daily_consumption: 2)).to eq(0)
    end
  end

  describe '#days_until_out_of_stock' do
    it 'returns nil when current supply is nil' do
      supply_level = described_class.new(current: nil, reorder_threshold: 10, last_restock: nil)

      expect(supply_level.days_until_out_of_stock(daily_consumption: 2)).to be_nil
    end

    it 'rounds up fractional days remaining' do
      supply_level = described_class.new(current: 5, reorder_threshold: 1, last_restock: 10)

      expect(supply_level.days_until_out_of_stock(daily_consumption: 2)).to eq(3)
    end
  end
end

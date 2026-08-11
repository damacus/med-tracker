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

  describe '#crossed_low_stock_threshold_from?' do
    it 'returns true when supply crosses from above to the reorder threshold' do
      supply_level = described_class.new(current: 10, reorder_threshold: 10, last_restock: 50)

      expect(supply_level.crossed_low_stock_threshold_from?(previous_current: 10.000000000000002)).to be true
    end

    it 'returns true when supply crosses from above to below the reorder threshold' do
      supply_level = described_class.new(current: 9, reorder_threshold: 10, last_restock: 50)

      expect(supply_level.crossed_low_stock_threshold_from?(previous_current: 11)).to be true
    end

    it 'returns false when supply remains above the reorder threshold' do
      supply_level = described_class.new(current: 11, reorder_threshold: 10, last_restock: 50)

      expect(supply_level.crossed_low_stock_threshold_from?(previous_current: 12)).to be false
    end

    it 'returns false when supply was already at the reorder threshold' do
      supply_level = described_class.new(current: 9, reorder_threshold: 10, last_restock: 50)

      expect(supply_level.crossed_low_stock_threshold_from?(previous_current: 10)).to be false
    end

    it 'returns false when supply is not tracked' do
      supply_level = described_class.new(current: nil, reorder_threshold: 10, last_restock: nil)

      expect(supply_level.crossed_low_stock_threshold_from?(previous_current: 11)).to be false
    end

    it 'returns false when the previous supply is unavailable' do
      supply_level = described_class.new(current: 10, reorder_threshold: 10, last_restock: 50)

      expect(supply_level.crossed_low_stock_threshold_from?(previous_current: nil)).to be false
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

    it 'subtracts the reorder reserve and rounds partial days up' do
      supply_level = described_class.new(current: 15, reorder_threshold: 10, last_restock: 50)

      expect(supply_level.days_until_low_stock(daily_consumption: 2)).to eq(3)
    end

    it 'does not add a day when decimal consumption divides the reserve exactly' do
      supply_level = described_class.new(
        current: BigDecimal('0.07'),
        reorder_threshold: 0,
        last_restock: 1
      )

      expect(supply_level.days_until_low_stock(daily_consumption: 0.01)).to eq(7)
    end

    it 'returns zero when already low stock' do
      supply_level = described_class.new(current: 10, reorder_threshold: 10, last_restock: 50)

      expect(supply_level.days_until_low_stock(daily_consumption: 2)).to eq(0)
    end

    it 'returns zero when supply is below the reorder threshold' do
      supply_level = described_class.new(current: 5, reorder_threshold: 10, last_restock: 50)

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

# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MedicationTakeStockMutation do
  subject(:mutation) { described_class.new(take, decrementer: decrementer) }

  let(:inventory) { instance_double(Medication) }
  let(:take) do
    instance_double(MedicationTake, inventory_medication: inventory)
  end
  let(:decrementer) { instance_double(MedicationTakeStockDecrement) }
  let(:stock_source) { instance_double(MedicationTakeStockSource) }

  before do
    allow(MedicationTakeStockSource).to receive(:new).and_return(stock_source)
  end

  describe '#decrement' do
    context 'when stock is not tracked' do
      before { allow(stock_source).to receive(:tracked?).and_return(false) }

      it 'returns nil' do
        expect(mutation.decrement).to be_nil
      end
    end

    context 'when stock is tracked' do
      let(:stock_row) { instance_double(Object) }

      before do
        allow(stock_source).to receive_messages(tracked?: true, inventory: inventory)
        allow(decrementer).to receive(:call).with(stock_source).and_return(stock_row)
      end

      it 'returns a StockChange with inventory and stock_row' do
        result = mutation.decrement
        expect(result).to be_a(described_class::StockChange)
        expect(result.inventory).to eq(inventory)
        expect(result.stock_row).to eq(stock_row)
      end
    end
  end

  describe '#inventory_matches_selected_dose?' do
    before { allow(stock_source).to receive(:selected_dose?).and_return(true) }

    it 'delegates to a stock_source for the given inventory' do
      expect(mutation.inventory_matches_selected_dose?(inventory)).to be true
    end
  end

  describe '#inventory_in_stock?' do
    before { allow(stock_source).to receive(:in_stock?).and_return(false) }

    it 'delegates to the primary stock_source' do
      expect(mutation.inventory_in_stock?).to be false
    end
  end
end

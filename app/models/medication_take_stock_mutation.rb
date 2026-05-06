# frozen_string_literal: true

class MedicationTakeStockMutation
  StockChange = Data.define(:inventory, :stock_row)

  def initialize(take, decrementer: MedicationTakeStockDecrement.new(take))
    @take = take
    @decrementer = decrementer
  end

  def decrement
    return unless stock_source.tracked?

    StockChange.new(inventory: stock_source.inventory, stock_row: decrementer.call(stock_source))
  end

  def inventory_matches_selected_dose?(inventory)
    stock_source_for(inventory).selected_dose?
  end

  def inventory_in_stock?
    stock_source.in_stock?
  end

  private

  attr_reader :take, :decrementer

  def stock_source
    @stock_source ||= stock_source_for(take.inventory_medication)
  end

  def stock_source_for(inventory)
    MedicationTakeStockSource.new(take: take, inventory: inventory)
  end
end

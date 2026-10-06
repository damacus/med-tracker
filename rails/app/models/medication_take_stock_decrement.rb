# frozen_string_literal: true

class MedicationTakeStockDecrement
  def initialize(take)
    @take = take
  end

  def call(stock_source)
    dosage_option = stock_source.dosage_option
    return decrement_dosage_option(stock_source.inventory, dosage_option) if dosage_option&.current_supply.present?

    decrement_inventory(stock_source.inventory, take.dose_unit)
  end

  private

  attr_reader :take

  def decrement_inventory(inventory, dose_unit)
    inventory.with_lock do
      previous_current_supply = inventory.current_supply
      current_supply = decremented_supply(previous_current_supply, dose_unit)
      inventory.paper_trail_event = 'dose_decrement'
      inventory.update!(current_supply: current_supply)
      stock_row(inventory, previous_current_supply)
    end
  end

  def decrement_dosage_option(inventory, dosage_option)
    decrement_dosage_option_current_supply(dosage_option)
    sync_inventory(inventory)
  end

  def decrement_dosage_option_current_supply(dosage_option)
    dosage_option.with_lock do
      dosage_option.with_inventory_sync_suppressed do
        dosage_option.update!(current_supply: decremented_supply(dosage_option.current_supply, dosage_option.unit))
      end
    end
  end

  def sync_inventory(inventory)
    inventory.with_lock do
      previous_current_supply = inventory.current_supply
      inventory.paper_trail_event = 'dose_decrement'
      inventory.sync_inventory_from_dosage_records!
      inventory.reload
      stock_row(inventory, previous_current_supply)
    end
  end

  def decremented_supply(current_supply, dose_unit)
    # Row locks reload stale records before decrement; the clamp keeps validation-bypass races at zero.
    [current_supply.to_d - stock_decrement_for(dose_unit: dose_unit), BigDecimal('0')].max
  end

  def stock_row(inventory, previous_current_supply)
    {
      'previous_current_supply' => previous_current_supply,
      'current_supply' => inventory.current_supply,
      'reorder_threshold' => inventory.reorder_threshold
    }
  end

  def stock_decrement_for(dose_unit:)
    MedicationStockConsumption.quantity_for(dose_amount: take.dose_amount, dose_unit: dose_unit)
  end
end

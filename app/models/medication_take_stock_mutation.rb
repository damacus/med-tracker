# frozen_string_literal: true

class MedicationTakeStockMutation
  StockChange = Data.define(:inventory, :stock_row)

  def initialize(take)
    @take = take
  end

  def decrement
    inventory = take.inventory_medication
    return if inventory.blank?
    return if inventory.current_supply.blank? && matching_inventory_dosage_option(inventory).blank?

    StockChange.new(inventory: inventory, stock_row: decrement_inventory_stock(inventory))
  end

  def inventory_matches_selected_dose?(inventory)
    return true unless inventory_dosage_option_resolver(inventory).tracked_inventory?

    matching_inventory_dosage_option(inventory).present?
  end

  def inventory_in_stock?
    inventory = take.inventory_medication
    return true if inventory.blank?
    return true if tracked_inventory_dosage_option_missing?(inventory)

    dosage_option = tracked_inventory_dosage_option(inventory)
    return inventory_option_in_stock?(inventory) if dosage_option&.current_supply.present?
    return true if inventory.current_supply.blank?

    MedicationStockConsumption.sufficient?(
      current_supply: inventory.current_supply,
      dose_amount: take.dose_amount,
      dose_unit: take.dose_unit
    )
  end

  private

  attr_reader :take

  def decrement_inventory_stock(inventory)
    dosage_option = matching_inventory_dosage_option(inventory)
    return decrement_dosage_option_stock(inventory, dosage_option) if dosage_option&.current_supply.present?

    inventory.with_lock do
      previous_current_supply = inventory.current_supply
      decrement = stock_decrement_for(dose_unit: take.dose_unit)
      current_supply = [previous_current_supply.to_d - decrement, BigDecimal('0')].max

      inventory.update!(current_supply: current_supply)

      {
        'previous_current_supply' => previous_current_supply,
        'current_supply' => current_supply,
        'reorder_threshold' => inventory.reorder_threshold
      }
    end
  end

  def decrement_dosage_option_stock(inventory, dosage_option)
    decrement_dosage_option_current_supply(dosage_option)

    inventory.with_lock do
      previous_current_supply = inventory.current_supply

      inventory.sync_inventory_from_dosage_records!
      inventory.reload

      {
        'previous_current_supply' => previous_current_supply,
        'current_supply' => inventory.current_supply,
        'reorder_threshold' => inventory.reorder_threshold
      }
    end
  end

  def decrement_dosage_option_current_supply(dosage_option)
    dosage_option.with_lock do
      dosage_option.with_inventory_sync_suppressed do
        decrement = stock_decrement_for(dose_unit: dosage_option.unit)
        current_supply = [dosage_option.current_supply.to_d - decrement, BigDecimal('0')].max

        dosage_option.update!(current_supply: current_supply)
      end
    end
  end

  def matching_inventory_dosage_option(inventory)
    inventory_dosage_option_resolver(inventory).call
  end

  def inventory_option_in_stock?(inventory)
    dosage_option = tracked_inventory_dosage_option(inventory)
    return true if dosage_option.blank? || dosage_option.current_supply.blank?

    MedicationStockConsumption.sufficient?(
      current_supply: dosage_option.current_supply,
      dose_amount: take.dose_amount,
      dose_unit: dosage_option.unit
    )
  end

  def tracked_inventory_dosage_option(inventory)
    matching_inventory_dosage_option(inventory) if inventory_dosage_option_resolver(inventory).tracked_inventory?
  end

  def tracked_inventory_dosage_option_missing?(inventory)
    inventory_dosage_option_resolver(inventory).tracked_inventory? && tracked_inventory_dosage_option(inventory).blank?
  end

  def inventory_dosage_option_resolver(inventory)
    InventoryDosageOptionResolver.new(inventory: inventory, source: take.source, effective_date: take.taken_at&.to_date)
  end

  def stock_decrement_for(dose_unit:)
    MedicationStockConsumption.quantity_for(dose_amount: take.dose_amount, dose_unit: dose_unit)
  end
end

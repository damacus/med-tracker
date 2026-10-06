# frozen_string_literal: true

class MedicationTakeStockSource
  attr_reader :take, :inventory

  def initialize(take:, inventory:)
    @take = take
    @inventory = inventory
  end

  def tracked?
    inventory.present? && (inventory.current_supply.present? || dosage_option.present?)
  end

  def selected_dose?
    return true if inventory.blank?
    return true unless tracked_dosage_inventory?

    dosage_option.present?
  end

  def in_stock?
    return true if inventory.blank?
    return true if missing_tracked_dose?
    return option_in_stock? if dosage_option&.current_supply.present?
    return true if inventory.current_supply.blank?

    sufficient?(inventory.current_supply, take.dose_unit)
  end

  def dosage_option
    return @dosage_option if defined?(@dosage_option)

    @dosage_option = resolver.call
  end

  private

  def option_in_stock?
    sufficient?(dosage_option.current_supply, dosage_option.unit)
  end

  def missing_tracked_dose?
    tracked_dosage_inventory? && dosage_option.blank?
  end

  def tracked_dosage_inventory?
    resolver.tracked_inventory?
  end

  def sufficient?(current_supply, dose_unit)
    MedicationStockConsumption.sufficient?(
      current_supply: current_supply,
      dose_amount: take.dose_amount,
      dose_unit: dose_unit
    )
  end

  def resolver
    @resolver ||= InventoryDosageOptionResolver.new(
      inventory: inventory,
      source: take.source,
      effective_date: take.taken_at&.to_date
    )
  end
end

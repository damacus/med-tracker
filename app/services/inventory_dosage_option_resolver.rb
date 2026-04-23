# frozen_string_literal: true

class InventoryDosageOptionResolver
  attr_reader :inventory, :source

  def initialize(inventory:, source:)
    @inventory = inventory
    @source = source
  end

  def tracked_inventory?
    tracked_inventory_dosage_records.any?
  end

  def call
    return if inventory.blank? || source.blank?
    return if tracked_inventory_dosage_records.empty?

    return resolved_from_source_dosage_option if source_dosage_option.present?

    resolved_from_source_snapshot
  end

  private

  def resolved_from_source_dosage_option
    return if source_dosage_option.blank?

    return matching_inventory_record_by_id if source_dosage_option.medication_id == inventory.id

    matches = tracked_inventory_dosage_records.select(&method(:matching_source_dosage_signature?))
    matches.one? ? matches.first : nil
  end

  def resolved_from_source_snapshot
    return if source.default_dose_amount.blank? || source.dose_unit.blank?

    matches = tracked_inventory_dosage_records.select(&method(:matching_source_snapshot?))
    matches.one? ? matches.first : nil
  end

  def tracked_inventory_dosage_records
    @tracked_inventory_dosage_records ||= inventory.dosage_records.where.not(current_supply: nil).to_a
  end

  def source_dosage_option
    return unless source.respond_to?(:source_dosage_option)

    source.source_dosage_option
  end

  def matching_inventory_record_by_id
    tracked_inventory_dosage_records.find { |dosage_option| dosage_option.id == source_dosage_option.id }
  end

  def matching_source_dosage_signature?(dosage_option)
    dosage_option.inventory_match_signature == source_dosage_option.inventory_match_signature
  end

  def matching_source_snapshot?(dosage_option)
    dosage_option.amount.to_s == source.default_dose_amount.to_s && dosage_option.unit == source.dose_unit
  end
end

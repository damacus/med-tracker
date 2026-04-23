# frozen_string_literal: true

# MedicationTake records when a dose of medication was administered
class MedicationTake < ApplicationRecord
  include OtelInstrumented

  belongs_to :schedule, optional: true
  belongs_to :person_medication, optional: true
  belongs_to :taken_from_medication, class_name: 'Medication', optional: true
  belongs_to :taken_from_location, class_name: 'Location', optional: true

  # CRITICAL: Audit trail for medication safety
  # Every dose must be tracked for legal/clinical compliance
  # Tracks: all medication administrations, timing, amounts
  # Cannot be edited or deleted - immutable record for patient safety
  # @see docs/audit-trail.md
  has_paper_trail

  validates :taken_at, presence: true
  validates :amount_ml, presence: true, numericality: { greater_than: 0 }
  validate :exactly_one_source
  validate :taken_from_medication_matches_source
  validate :taken_from_medication_matches_selected_dose
  validate :taken_from_location_matches_medication
  validate :taken_from_medication_is_in_stock

  before_validation :assign_taken_from_location
  after_create :decrement_medication_stock
  after_commit :publish_low_stock_threshold_reached, on: :create

  # Delegate to get the source (schedule or person_medication)
  def source
    schedule || person_medication
  end

  def person
    schedule&.person || person_medication&.person
  end

  def medication
    schedule&.medication || person_medication&.medication
  end

  def inventory_medication
    taken_from_medication || medication
  end

  def inventory_location
    taken_from_location || taken_from_medication&.location || medication&.location
  end

  private

  def decrement_medication_stock
    inventory = inventory_medication
    return unless inventory
    return if inventory.current_supply.blank? && matching_inventory_dosage_option(inventory).blank?

    stock_row = decrement_inventory_stock(inventory)
    return unless stock_row

    remember_low_stock_threshold_crossing(inventory:, stock_row:)
  end

  def exactly_one_source
    sources = [schedule_id, person_medication_id].compact
    return if sources.one?

    errors.add(:base, 'Must have exactly one source (schedule or person_medication)')
  end

  def assign_taken_from_location
    self.taken_from_location ||= taken_from_medication&.location
  end

  def taken_from_medication_matches_source
    return if taken_from_medication.blank? || medication.blank?
    return if taken_from_medication.name == medication.name &&
              taken_from_medication.dosage_amount.to_s == medication.dosage_amount.to_s &&
              taken_from_medication.dosage_unit == medication.dosage_unit

    errors.add(:taken_from_medication, 'must match the assigned medication')
  end

  def taken_from_medication_matches_selected_dose
    return if taken_from_medication.blank?
    return unless inventory_dosage_option_resolver(taken_from_medication).tracked_inventory?
    return if matching_inventory_dosage_option(taken_from_medication).present?

    errors.add(:taken_from_medication, 'must include stock for the selected dose')
  end

  def taken_from_location_matches_medication
    return if taken_from_medication.blank? && taken_from_location.blank?
    return if taken_from_medication&.location == taken_from_location

    errors.add(:taken_from_location, 'must match the selected medication location')
  end

  def taken_from_medication_is_in_stock
    return if taken_from_medication.blank?
    return if tracked_inventory_dosage_option_missing?(taken_from_medication)
    return if inventory_option_in_stock?(taken_from_medication)

    if tracked_inventory_dosage_option(taken_from_medication)&.current_supply.present?
      errors.add(:taken_from_medication, 'must be in stock')
      return
    end
    return unless taken_from_medication.out_of_stock?

    errors.add(:taken_from_medication, 'must be in stock')
  end

  # Custom OpenTelemetry span attributes for medication tracking
  def otel_span_attributes(operation)
    attrs = {
      'model.name' => self.class.name,
      'model.id' => id.to_s,
      'model.operation' => operation,
      'medication_take.taken_at' => taken_at&.iso8601
    }

    # Add source-specific attributes
    if schedule_id
      attrs['medication_take.source_type'] = 'schedule'
      attrs['medication_take.schedule_id'] = schedule_id.to_s
    elsif person_medication_id
      attrs['medication_take.source_type'] = 'person_medication'
      attrs['medication_take.person_medication_id'] = person_medication_id.to_s
    end

    attrs.merge(taken_from_span_attributes)
  end

  def taken_from_span_attributes
    return {} unless taken_from_medication_id

    attrs = { 'medication_take.taken_from_medication_id' => taken_from_medication_id.to_s }
    attrs['medication_take.taken_from_location_id'] = taken_from_location_id.to_s if taken_from_location_id
    attrs
  end

  def decrement_inventory_stock(inventory)
    dosage_option = matching_inventory_dosage_option(inventory)
    return decrement_dosage_option_stock(inventory, dosage_option) if dosage_option&.current_supply.present?

    inventory.with_lock do
      previous_current_supply = inventory.current_supply
      current_supply = [previous_current_supply.to_i - 1, 0].max

      inventory.update!(current_supply: current_supply)

      {
        'previous_current_supply' => previous_current_supply,
        'current_supply' => current_supply,
        'reorder_threshold' => inventory.reorder_threshold
      }
    end
  end

  def decrement_dosage_option_stock(inventory, dosage_option)
    previous_current_supply = nil

    inventory.with_lock do
      previous_current_supply = inventory.current_supply

      dosage_option.with_lock do
        dosage_option.update!(current_supply: [dosage_option.current_supply.to_i - 1, 0].max)
      end

      inventory.sync_inventory_from_dosage_records!
    end

    inventory.reload

    {
      'previous_current_supply' => previous_current_supply,
      'current_supply' => inventory.current_supply,
      'reorder_threshold' => inventory.reorder_threshold
    }
  end

  def remember_low_stock_threshold_crossing(inventory:, stock_row:)
    return unless low_stock_threshold_crossed?(inventory:, stock_row:)

    @low_stock_threshold_payload = low_stock_threshold_payload(inventory:, stock_row:)
  end

  def publish_low_stock_threshold_reached
    return unless @low_stock_threshold_payload

    ActiveSupport::Notifications.instrument(
      'low_stock_threshold_reached.med_tracker',
      @low_stock_threshold_payload
    )
  ensure
    @low_stock_threshold_payload = nil
  end

  def low_stock_threshold_crossed?(inventory:, stock_row:)
    SupplyLevel.new(
      current: stock_row['current_supply'],
      reorder_threshold: stock_row['reorder_threshold'],
      last_restock: inventory.supply_at_last_restock
    ).crossed_low_stock_threshold_from?(previous_current: stock_row['previous_current_supply'])
  end

  def low_stock_threshold_payload(inventory:, stock_row:)
    {
      medication_id: inventory.id,
      location_id: inventory.location_id,
      take_id: id,
      source_type: schedule_id.present? ? 'schedule' : 'person_medication',
      source_id: schedule_id || person_medication_id,
      previous_current_supply: stock_row['previous_current_supply'],
      current_supply: stock_row['current_supply'],
      reorder_threshold: stock_row['reorder_threshold'],
      taken_at: taken_at
    }
  end

  def matching_inventory_dosage_option(inventory)
    inventory_dosage_option_resolver(inventory).call
  end

  def inventory_option_in_stock?(inventory)
    dosage_option = tracked_inventory_dosage_option(inventory)
    return true if dosage_option.blank? || dosage_option.current_supply.blank?

    dosage_option.current_supply.to_i.positive?
  end

  def tracked_inventory_dosage_option(inventory)
    matching_inventory_dosage_option(inventory) if inventory_dosage_option_resolver(inventory).tracked_inventory?
  end

  def tracked_inventory_dosage_option_missing?(inventory)
    inventory_dosage_option_resolver(inventory).tracked_inventory? && tracked_inventory_dosage_option(inventory).blank?
  end

  def inventory_dosage_option_resolver(inventory)
    InventoryDosageOptionResolver.new(inventory: inventory, source: source)
  end
end

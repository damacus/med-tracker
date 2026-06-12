# frozen_string_literal: true

# MedicationTake records when a dose of medication was administered
class MedicationTake < ApplicationRecord
  include OtelInstrumented

  belongs_to :household, optional: true
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
  validates :dose_amount, presence: true, numericality: { greater_than: 0 }
  validates :dose_unit, presence: true
  validates :client_uuid, uniqueness: true, allow_blank: true
  validate :exactly_one_source
  validate :taken_from_medication_matches_source
  validate :taken_from_medication_matches_selected_dose
  validate :taken_from_location_matches_medication
  validate :taken_from_medication_is_in_stock

  before_validation :assign_household
  before_validation :assign_taken_from_location
  before_validation :assign_dose_snapshot
  after_create :decrement_medication_stock
  after_commit :publish_low_stock_threshold_reached, on: :create

  # Delegate to get the source (schedule or person_medication)
  def source
    schedule || person_medication
  end

  def source_type
    schedule_id.present? ? 'schedule' : 'person_medication'
  end

  def source_record_id
    schedule_id || person_medication_id
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

  def assign_household
    self.household ||= source_household || inventory_household
  end

  def source_household
    schedule&.household || person_medication&.household
  end

  def inventory_household
    taken_from_medication&.household || taken_from_location&.household
  end

  def decrement_medication_stock
    stock_change = stock_mutation.decrement
    return unless stock_change

    remember_low_stock_threshold_crossing(inventory: stock_change.inventory, stock_row: stock_change.stock_row)
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
    return if stock_mutation.inventory_matches_selected_dose?(taken_from_medication)

    errors.add(:taken_from_medication, 'must include stock for the selected dose')
  end

  def taken_from_location_matches_medication
    return if taken_from_medication.blank? && taken_from_location.blank?
    return if taken_from_medication&.location == taken_from_location

    errors.add(:taken_from_location, 'must match the selected medication location')
  end

  def taken_from_medication_is_in_stock
    return if stock_mutation.inventory_in_stock?

    errors.add(:taken_from_medication, 'must be in stock')
  end

  # Custom OpenTelemetry span attributes for medication tracking.
  # Keep medication administration spans intentionally coarse: dose timing,
  # quantities, units, and related record identifiers can reveal PHI when traces
  # leave the application boundary.
  def otel_span_attributes(operation)
    {
      'model.name' => self.class.name,
      'model.operation' => operation
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
      source_type: source_type,
      source_id: source_record_id,
      previous_current_supply: stock_row['previous_current_supply'],
      current_supply: stock_row['current_supply'],
      reorder_threshold: stock_row['reorder_threshold'],
      taken_at: taken_at
    }
  end

  def stock_mutation
    @stock_mutation ||= MedicationTakeStockMutation.new(self)
  end

  def assign_dose_snapshot
    self.dose_amount ||= source_dose_amount
    self.dose_unit ||= source_dose_unit
  end

  def source_dose_amount
    return source.effective_dose_amount(effective_date) if source.respond_to?(:effective_dose_amount)

    source&.default_dose_amount
  end

  def source_dose_unit
    return source.effective_dose_unit(effective_date) if source.respond_to?(:effective_dose_unit)

    source&.dose_unit
  end

  def effective_date
    return taken_at.to_date if taken_at.respond_to?(:to_date)

    Time.zone.today
  end
end

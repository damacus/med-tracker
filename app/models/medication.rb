# frozen_string_literal: true

class Medication < ApplicationRecord # :nodoc:
  DOSAGE_UNITS = %w[tablet mg ml g mcg IU spray drop sachet].freeze
  CATEGORIES = [
    'Analgesic',
    'Antibiotic',
    'Anticoagulant',
    'Anticonvulsant',
    'Antidepressant',
    'Antidiabetic',
    'Antiemetic',
    'Antifungal',
    'Antihistamine',
    'Antihypertensive',
    'Anti-Inflammatory',
    'Antiparasitic',
    'Antipsychotic',
    'Antiviral',
    'Anxiolytic',
    'Cardiovascular',
    'Cholesterol',
    'Contraceptive',
    'Dermatological',
    'Gastrointestinal',
    'Hormonal',
    'Immunosuppressant',
    'Migraine',
    'Mineral',
    'Muscle Relaxant',
    'Neurological',
    'Oncology',
    'Ophthalmic',
    'Osmotic Laxative',
    'Opioid',
    'Osteoporosis',
    'Respiratory',
    'Sleep Aid',
    'Smoking Cessation',
    'Supplement',
    'Thyroid',
    'Urological',
    'Vitamin',
    'Weight Management'
  ].freeze

  has_paper_trail if: proc { |medication| medication.paper_trail_event.present? }

  belongs_to :location

  has_many :dosage_records, class_name: 'MedicationDosageOption', dependent: :destroy, inverse_of: :medication
  has_many :schedules, dependent: :destroy
  has_many :person_medications, dependent: :destroy

  accepts_nested_attributes_for :dosage_records, allow_destroy: true, reject_if: :blank_dosage_record_attributes?

  validate :single_dose_switch_requires_no_schedules
  validate :nested_dosage_records_are_valid
  after_commit :sync_dosages, on: :update

  enum :reorder_status, { requested: 0, ordered: 1, received: 2 }, prefix: :reorder

  validates :barcode, uniqueness: true, allow_blank: true
  validates :name, presence: true
  validates :category, inclusion: { in: CATEGORIES }, allow_blank: true
  validates :dosage_amount, numericality: { greater_than: 0 }, allow_nil: true
  validates :dosage_unit, inclusion: { in: DOSAGE_UNITS }, allow_blank: true
  validates :current_supply, numericality: { only_integer: true, greater_than_or_equal_to: 0 }, allow_nil: true
  validates :supply_at_last_restock, numericality: { only_integer: true, greater_than_or_equal_to: 0 }, allow_nil: true
  validates :reorder_threshold, presence: true, numericality: { only_integer: true, greater_than_or_equal_to: 0 }

  delegate :low_stock?, :out_of_stock?, to: :supply_level

  def sync_dosages
    return unless persisted? && switched_to_single_dose_mode?

    # ⚡ Bolt Optimization: Use `.any?` instead of `.exists?`
    # This avoids a redundant COUNT/EXISTS query if `schedules` is already loaded in memory
    return if schedules.any?

    # When switching to single-dose mode (dosage_amount is set),
    # remove all orphaned multi-dose records to prevent data pollution.
    # Uses SQL to comply with RuboCop and project standards.
    stmt = 'DELETE FROM dosages WHERE medication_id = $1'
    binds = [
      ActiveRecord::Relation::QueryAttribute.new('medication_id', id, ActiveRecord::Type::BigInteger.new)
    ]
    ActiveRecord::Base.connection.exec_delete(stmt, 'Sync Dosages', binds)
  end

  def restock!(quantity:) # rubocop:disable Naming/PredicateMethod
    increment = quantity.to_i
    return false if increment <= 0

    with_lock do
      new_supply = current_supply.to_i + increment
      update!(
        current_supply: new_supply,
        supply_at_last_restock: new_supply,
        reorder_status: nil,
        ordered_at: nil,
        reordered_at: nil
      )
    end

    true
  end

  def supply_percentage
    supply_level.percentage
  end

  def estimated_daily_consumption
    return @estimated_daily_consumption if defined?(@estimated_daily_consumption)

    # ⚡ Bolt Optimization: Use Enumerable#select instead of ActiveRecord#active
    # to filter schedules in-memory. This prevents N+1 queries when calculating
    # daily consumption for a collection of eager-loaded medications.
    schedule_rate = schedules.select(&:active?).sum do |schedule|
      next 0.0 if schedule.max_daily_doses.blank?

      schedule.max_daily_doses.to_f / (schedule.cycle_period / 1.day)
    end

    pm_rate = person_medications.sum do |pm|
      next 0.0 if pm.max_daily_doses.blank?

      pm.max_daily_doses.to_f
    end

    @estimated_daily_consumption = schedule_rate + pm_rate
  end

  def forecast_available?
    supply_level.forecast_available?(daily_consumption: estimated_daily_consumption)
  end

  def days_until_out_of_stock
    supply_level.days_until_out_of_stock(daily_consumption: estimated_daily_consumption)
  end

  def days_until_low_stock
    supply_level.days_until_low_stock(daily_consumption: estimated_daily_consumption)
  end

  def default_dosage_for_person_type(person_type)
    child_person_type?(person_type) ? child_default_dosage : adult_default_dosage
  end

  def out_of_stock_date
    days = days_until_out_of_stock
    days ? Time.zone.today + days.days : nil
  end

  def low_stock_date
    days = days_until_low_stock
    days ? Time.zone.today + days.days : nil
  end

  def supply_level
    SupplyLevel.new(
      current: current_supply,
      reorder_threshold: reorder_threshold,
      last_restock: supply_at_last_restock
    )
  end

  def dosages
    dosage_records.order(:amount, :id).map(&:to_value)
  end

  def dose_options_payload
    dosages.map(&:to_option_payload)
  end

  def adult_default_dosage
    default_dosage(:default_for_adults?)
  end

  def child_default_dosage
    default_dosage(:default_for_children?)
  end

  alias dosage_for_person_type default_dosage_for_person_type

  private

  def default_dosage(predicate)
    loaded = dosages.to_a
    loaded.find(&predicate) || loaded.first
  end

  def child_person_type?(person_type)
    %w[minor dependent_adult].include?(person_type.to_s)
  end

  def blank_dosage_record_attributes?(attributes)
    ignored_keys = %w[id _destroy unit default_dose_cycle]
    meaningful_attributes = attributes.except(*ignored_keys)

    meaningful_attributes.values.all? do |value|
      value.blank? || value == '0'
    end
  end

  def single_dose_switch_requires_no_schedules
    return unless switching_to_single_dose_mode?

    # ⚡ Bolt Optimization: Use `.any?` instead of `.exists?`
    # This avoids a redundant COUNT/EXISTS query if `schedules` is already loaded in memory
    return unless schedules.any?

    errors.add(:dosage_amount,
               'cannot switch to a single standard dose while schedules still use dose options')
  end

  def switching_to_single_dose_mode?
    return false unless will_save_change_to_dosage_amount?

    previous_amount, new_amount = dosage_amount_change_to_be_saved
    previous_amount.blank? && new_amount.present?
  end

  def switched_to_single_dose_mode?
    change = previous_changes['dosage_amount']
    return false if change.blank?

    previous_amount, new_amount = change
    previous_amount.blank? && new_amount.present?
  end

  def nested_dosage_records_are_valid
    dosage_records.reject(&:marked_for_destruction?).each do |dosage_record|
      next if dosage_record.valid?

      dosage_record.errors.full_messages.each do |message|
        errors.add(:base, message)
      end
    end
  end
end

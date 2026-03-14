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

  has_many :dosages, dependent: :destroy
  has_many :schedules, dependent: :destroy
  has_many :person_medications, dependent: :destroy

  validate :single_dose_switch_requires_no_schedules
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

  def sync_dosages
    return unless persisted? && switched_to_single_dose_mode?
    return if schedules.exists?

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
    current = current_supply || 0
    denominator = supply_at_last_restock || [reorder_threshold, 1].max
    return 0 if denominator <= 0

    [current.to_f / denominator * 100, 100].min.round
  end

  def low_stock?
    return false if current_supply.nil?

    current_supply <= reorder_threshold
  end

  def out_of_stock?
    return false if current_supply.nil?

    current_supply <= 0
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
    current_supply.present? && estimated_daily_consumption.positive?
  end

  def days_until_out_of_stock
    return nil unless forecast_available?
    return 0 if out_of_stock?

    (current_supply.to_f / estimated_daily_consumption).ceil
  end

  def days_until_low_stock
    return nil unless forecast_available?
    return 0 if low_stock?

    surplus = current_supply - reorder_threshold
    return 0 if surplus <= 0

    (surplus.to_f / estimated_daily_consumption).ceil
  end

  def default_dosage_for_person_type(person_type)
    child_types = %w[minor dependent_adult]
    if child_types.include?(person_type.to_s)
      dosages.to_a.find(&:default_for_children) || dosages.to_a.first
    else
      dosages.to_a.find(&:default_for_adults) || dosages.to_a.first
    end
  end

  def out_of_stock_date
    days = days_until_out_of_stock
    days ? Time.zone.today + days.days : nil
  end

  def low_stock_date
    days = days_until_low_stock
    days ? Time.zone.today + days.days : nil
  end

  private

  def single_dose_switch_requires_no_schedules
    return unless switching_to_single_dose_mode?
    return unless schedules.exists?

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
end

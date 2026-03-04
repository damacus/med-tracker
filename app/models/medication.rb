# frozen_string_literal: true

class Medication < ApplicationRecord # :nodoc:
  DOSAGE_UNITS = %w[tablet mg ml g mcg IU spray drop sachet].freeze
  PRESET_DOSAGE_AMOUNTS = %w[0.5 1 1.5 2 3 4].freeze
  MEDICAL_UNITS_NO_PLURALIZATION = %w[mg ml g mcg IU].freeze
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

  enum :reorder_status, { requested: 0, ordered: 1, received: 2 }, prefix: :reorder

  attr_accessor :dosage_presets, :single_dose

  validates :name, presence: true
  validates :category, inclusion: { in: CATEGORIES }, allow_blank: true
  validates :dosage_amount, numericality: { greater_than: 0 }, allow_nil: true
  validates :dosage_unit, presence: true, inclusion: { in: DOSAGE_UNITS }
  validates :current_supply, numericality: { only_integer: true, greater_than_or_equal_to: 0 }, allow_nil: true
  validates :supply_at_last_restock, numericality: { only_integer: true, greater_than_or_equal_to: 0 }, allow_nil: true
  validates :reorder_threshold, presence: true, numericality: { only_integer: true, greater_than_or_equal_to: 0 }

  after_save :sync_dosages

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

  def supply_label(count: current_supply)
    c = count.to_i
    u = dosage_unit.presence || 'unit'

    label = if MEDICAL_UNITS_NO_PLURALIZATION.include?(u)
              u
            else
              c == 1 ? u : u.pluralize
            end

    "#{c} #{label}"
  end

  def out_of_stock?
    return false if current_supply.nil?

    current_supply <= 0
  end

  def estimated_daily_consumption
    schedule_rate = schedules.active.sum do |schedule|
      next 0.0 if schedule.max_daily_doses.blank?

      schedule.max_daily_doses.to_f / (schedule.cycle_period / 1.day)
    end

    pm_rate = person_medications.sum do |pm|
      next 0.0 if pm.max_daily_doses.blank?

      pm.max_daily_doses.to_f
    end

    schedule_rate + pm_rate
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
      dosages.find_by(default_for_children: true) || dosages.first
    else
      dosages.find_by(default_for_adults: true) || dosages.first
    end
  end

  def out_of_stock_date
    days = days_until_out_of_stock
    days ? Time.zone.today + days.days : nil
  end

  def assigned_people
    schedules_people = schedules.active.includes(:person).map(&:person)
    pm_people = person_medications.includes(:person).map(&:person)
    (schedules_people + pm_people).uniq
  end

  def low_stock_date
    days = days_until_low_stock
    days ? Time.zone.today + days.days : nil
  end

  private

  def sync_dosages
    return unless single_dose.present? || dosage_presets.present?

    if single_dose == 'true'
      sync_single_dosage
    elsif dosage_presets.present?
      sync_preset_dosages
    end
  end

  def sync_single_dosage
    return if dosage_amount.blank?

    dosages.where('amount != ? OR unit != ?', dosage_amount, dosage_unit).destroy_all
    dosages.find_or_create_by!(
      amount: dosage_amount,
      unit: dosage_unit,
      frequency: 'As prescribed',
      description: 'Standard dose'
    )
  end

  def sync_preset_dosages
    amounts = dosage_presets.split(',').map(&:to_f).uniq
    dosages.where('amount NOT IN (?) OR unit != ?', amounts, dosage_unit).destroy_all
    amounts.each do |amount|
      dosages.find_or_create_by!(
        amount: amount,
        unit: dosage_unit,
        frequency: 'As prescribed',
        description: 'Preset dose'
      )
    end
  end
end

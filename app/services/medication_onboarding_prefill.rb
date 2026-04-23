# frozen_string_literal: true

class MedicationOnboardingPrefill
  Result = Data.define(:medication_attributes, :dosage_records_attributes)

  DISPLAY_UNIT_MAP = {
    'tablet' => 'tablet',
    'tablets' => 'tablet',
    'capsule' => 'capsule',
    'capsules' => 'capsule',
    'sachet' => 'sachet',
    'sachets' => 'sachet',
    'spray' => 'spray',
    'sprays' => 'spray',
    'drop' => 'drop',
    'drops' => 'drop',
    'pad' => 'pad',
    'pads' => 'pad'
  }.freeze
  DEFAULT_TIMING = {
    frequency: 'As directed',
    default_for_adults: true,
    default_for_children: false,
    default_max_daily_doses: 1,
    default_min_hours_between_doses: 24,
    default_dose_cycle: 'daily'
  }.freeze
  DISCRETE_PACKAGE_UNITS = %w[tablet capsule sachet spray drop pad].freeze

  def call(barcode: nil, code: nil, name: nil, package_quantity: nil, package_unit: nil)
    curated_product = BarcodeCatalog::CuratedProducts.find(barcode: barcode, code: code, name: name)
    return result_from_curated_product(curated_product) if curated_product
    return result_from_package_metadata(package_quantity:, package_unit:) if package_quantity.present?

    result_from_dosages(parsed_dosages(name))
  end

  private

  def result_from_curated_product(curated_product)
    dosages = curated_product.dosage_attributes
    dosage_attributes = medication_attributes_from_dosages(dosages)

    Result.new(
      medication_attributes: dosage_attributes.merge(curated_product.medication_attributes),
      dosage_records_attributes: dosages
    )
  end

  def result_from_dosages(dosages)
    compact_dosages = dosages.compact
    Result.new(
      medication_attributes: medication_attributes_from_dosages(compact_dosages),
      dosage_records_attributes: compact_dosages
    )
  end

  def result_from_package_metadata(package_quantity:, package_unit:)
    quantity = normalize_package_quantity(package_quantity)
    return blank_result unless quantity
    return blank_result if package_unit.blank?

    normalized_unit = DISPLAY_UNIT_MAP.fetch(package_unit.to_s.downcase, package_unit.to_s.downcase)
    return supply_only_result(quantity) unless discrete_package_unit?(normalized_unit)

    result_from_dosages([package_dosage(unit: normalized_unit, quantity: quantity)])
  end

  def medication_attributes_from_dosages(dosages)
    attributes = dosage_strength_attributes(dosages)
    attributes.merge!(tracked_supply_attributes(dosages))
    attributes.merge!(tracked_reorder_threshold_attributes(dosages))
    attributes[:reorder_threshold] = 0 if attributes[:reorder_threshold].blank? && dosages.any?
    attributes
  end

  def dosage_strength_attributes(dosages)
    return {} if dosages.empty?

    units = dosages.filter_map { |dosage| dosage[:unit] }.uniq
    attributes = {}
    attributes[:dosage_unit] = units.first if units.one?
    return attributes unless dosages.one?

    attributes.merge(
      dosage_amount: dosages.first[:amount],
      dosage_unit: dosages.first[:unit]
    )
  end

  def tracked_supply_attributes(dosages)
    tracked_current_supply = dosages.filter_map { |dosage| dosage[:current_supply] }
    return {} unless tracked_current_supply.any?

    {
      current_supply: tracked_current_supply.sum,
      supply_at_last_restock: tracked_current_supply.sum
    }
  end

  def tracked_reorder_threshold_attributes(dosages)
    tracked_reorder_threshold = dosages.filter_map { |dosage| dosage[:reorder_threshold] }
    return {} unless tracked_reorder_threshold.any?

    {
      reorder_threshold: tracked_reorder_threshold.sum
    }
  end

  def parsed_dosages(name)
    return [] if name.blank?

    pack_counts = parse_pack_counts(name)
    units = parse_units(name)
    return [] if units.empty?

    if pack_counts.any?
      pack_counts.map do |unit, count|
        build_dosage(unit: unit, current_supply: count, reorder_threshold: count / 4)
      end
    elsif units.one?
      [build_dosage(unit: units.first)]
    else
      []
    end
  end

  def parse_pack_counts(name)
    pattern = /(\d+)\s+(tablets?|capsules?|sachets?|sprays?|drops?|pads?)\b/i

    name.to_s.scan(pattern).each_with_object({}) do |(count, raw_unit), counts|
      unit = DISPLAY_UNIT_MAP.fetch(raw_unit.downcase, nil)
      next if unit.blank?

      counts[unit] = counts.fetch(unit, 0) + count.to_i
    end
  end

  def parse_units(name)
    name.to_s.scan(/\b(tablets?|capsules?|sachets?|sprays?|drops?|pads?)\b/i)
        .flatten
        .map { |raw_unit| DISPLAY_UNIT_MAP.fetch(raw_unit.downcase, nil) }
        .compact
        .uniq
  end

  def build_dosage(unit:, current_supply: nil, reorder_threshold: nil)
    DEFAULT_TIMING.merge(
      amount: 1,
      unit: unit,
      current_supply: current_supply,
      reorder_threshold: reorder_threshold
    )
  end

  def normalize_package_quantity(quantity)
    numeric = quantity.to_s.strip.tr(',', '.')
    return nil if numeric.blank? || !numeric.match?(/\A\d+(?:\.\d+)?\z/)

    return numeric.to_i if numeric.match?(/\A\d+\z/)

    numeric.to_f
  end

  def package_dosage(unit:, quantity:)
    build_dosage(unit: unit, current_supply: quantity, reorder_threshold: quantity / 4)
  end

  def supply_only_result(quantity)
    Result.new(
      medication_attributes: {
        current_supply: quantity,
        supply_at_last_restock: quantity,
        reorder_threshold: quantity / 4
      },
      dosage_records_attributes: []
    )
  end

  def blank_result
    Result.new(medication_attributes: {}, dosage_records_attributes: [])
  end

  def discrete_package_unit?(unit)
    DISCRETE_PACKAGE_UNITS.include?(unit)
  end
end

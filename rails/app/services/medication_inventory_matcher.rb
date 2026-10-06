# frozen_string_literal: true

class MedicationInventoryMatcher
  STRENGTH_PATTERN = %r{
    \b
    (?<amount>\d+(?:\.\d+)?)
    \s*
    (?<unit>mg|mcg|micrograms?|g|iu)
    \b
    (?:
      \s*/\s*
      (?<denominator_amount>\d+(?:\.\d+)?)
      \s*
      (?<denominator_unit>ml|l)
      \b
    )?
  }ix
  PACK_COUNT_PATTERN = /\b\d+(?:\.\d+)?\s*(?:tablets?|capsules?|caplets?|sachets?|sprays?|drops?|pads?|ml|l)\b/i
  FORM_WORDS = %w[
    tablet tablets capsule capsules caplet caplets sachet sachets spray sprays drop drops pad pads
    oral solution suspension liquid powder
  ].freeze
  FORM_UNITS = {
    'tablet' => 'tablet',
    'tablets' => 'tablet',
    'capsule' => 'capsule',
    'capsules' => 'capsule',
    'caplet' => 'tablet',
    'caplets' => 'tablet',
    'sachet' => 'sachet',
    'sachets' => 'sachet',
    'spray' => 'spray',
    'sprays' => 'spray',
    'drop' => 'drop',
    'drops' => 'drop',
    'pad' => 'pad',
    'pads' => 'pad',
    'solution' => 'liquid',
    'suspension' => 'liquid',
    'liquid' => 'liquid'
  }.freeze
  STRENGTH_UNITS = %w[mg mcg g iu].freeze

  def initialize(scope:)
    @scope = scope
  end

  def call(candidate)
    exact_barcode_match(candidate) || exact_dmd_match(candidate) || compatible_name_match(candidate)
  end

  private

  attr_reader :scope

  def exact_barcode_match(candidate)
    barcode = candidate.barcode
    return nil if barcode.blank?

    scope.find_by(barcode: barcode)
  end

  def exact_dmd_match(candidate)
    dmd_code = candidate.dmd_code
    return nil if dmd_code.blank?

    scope.find_by(dmd_code: dmd_code)
  end

  def compatible_name_match(candidate)
    candidate_name_key = name_key(candidate)
    return nil if candidate_name_key.blank?

    scope.includes(:dosage_records).detect do |medication|
      same_name_key?(candidate_name_key, medication) &&
        compatible_strength?(candidate, medication) &&
        compatible_form?(candidate, medication)
    end
  end

  def same_name_key?(candidate_name_key, medication)
    candidate_name_key == name_key(medication)
  end

  def compatible_strength?(candidate, medication)
    candidate_strength = strength_key(candidate)
    medication_strength = strength_key(medication)

    return candidate_strength == medication_strength if candidate_strength.present? && medication_strength.present?

    candidate_strength.blank? && medication_strength.blank?
  end

  def compatible_form?(candidate, medication)
    candidate_form = form_key(candidate)
    medication_form = form_key(medication)

    return true if candidate_form.blank? || medication_form.blank?

    candidate_form == medication_form
  end

  def name_key(medication)
    medication.name.to_s
              .downcase
              .gsub(/\([^)]*\)/, ' ')
              .gsub(STRENGTH_PATTERN, ' ')
              .gsub(PACK_COUNT_PATTERN, ' ')
              .then { |name| remove_form_words(name) }
              .gsub(/[^a-z0-9]+/, ' ')
              .squish
  end

  def remove_form_words(name)
    words = FORM_WORDS.map { |word| Regexp.escape(word) }.join('|')
    name.gsub(/\b(?:#{words})\b/i, ' ')
  end

  def strength_key(medication)
    strength_from_name(medication.name) || strength_from_dosage(medication)
  end

  def strength_from_name(name)
    match = name.to_s.match(STRENGTH_PATTERN)
    return nil unless match

    amount = normalized_decimal(match[:amount])
    unit = normalized_strength_unit(match[:unit])
    denominator_amount = match[:denominator_amount]
    return "#{amount} #{unit}" if denominator_amount.blank?

    denominator_amount = normalized_decimal(denominator_amount)
    denominator_unit = match[:denominator_unit].downcase
    "#{amount} #{unit}/#{denominator_amount} #{denominator_unit}"
  end

  def strength_from_dosage(medication)
    unit = normalized_strength_unit(medication.dose_unit)
    return nil unless STRENGTH_UNITS.include?(unit)

    dose_amount = medication.dose_amount
    return nil if dose_amount.blank?

    "#{normalized_decimal(dose_amount)} #{unit}"
  end

  def normalized_strength_unit(unit)
    normalized_unit = unit.to_s.downcase
    return 'mcg' if normalized_unit.start_with?('microgram')

    normalized_unit
  end

  def form_key(medication)
    normalized_unit = FORM_UNITS[medication.dose_unit.to_s.downcase]
    return normalized_unit if normalized_unit.present?

    name_form_key(medication.name)
  end

  def name_form_key(name)
    FORM_UNITS.each do |word, form|
      return form if name.to_s.match?(/\b#{Regexp.escape(word)}\b/i)
    end

    nil
  end

  def normalized_decimal(value)
    BigDecimal(value.to_s).to_s('F').sub(/\.?0+\z/, '')
  end
end

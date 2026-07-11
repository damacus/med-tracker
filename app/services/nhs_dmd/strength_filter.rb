# frozen_string_literal: true

require 'bigdecimal'

module NhsDmd
  class StrengthFilter
    UNIT_ALIASES = {
      'microgram' => 'mcg', 'micrograms' => 'mcg', 'mcg' => 'mcg', 'ug' => 'mcg', 'μg' => 'mcg',
      'milligram' => 'mg', 'milligrams' => 'mg', 'mg' => 'mg',
      'gram' => 'g', 'grams' => 'g', 'g' => 'g',
      'millilitre' => 'ml', 'millilitres' => 'ml', 'milliliter' => 'ml', 'milliliters' => 'ml', 'ml' => 'ml',
      'litre' => 'l', 'litres' => 'l', 'liter' => 'l', 'liters' => 'l', 'l' => 'l'
    }.freeze
    UNIT_PATTERN = UNIT_ALIASES.keys.sort_by { |unit| -unit.length }.join('|')
    NUMBER_PATTERN = '\\d[\\d,.]*(?:\\.\\d+)?'
    STRENGTH_PATTERN = %r{#{NUMBER_PATTERN}\s*(?:#{UNIT_PATTERN})(?:\s*/\s*#{NUMBER_PATTERN}\s*(?:#{UNIT_PATTERN}))?}i
    COMPONENT_PATTERN = /\A(\d+(?:\.\d+)?)\s*(#{UNIT_PATTERN})\z/i

    def self.normalize(value)
      components = value.to_s.downcase.tr(',', '').split('/').map(&:strip)
      return if components.empty? || components.length > 2

      normalized = components.map { |component| normalize_component(component) }
      normalized.join('/') if normalized.all?
    end

    def self.filter(results, strength)
      normalized_strength = normalize(strength)
      return results if normalized_strength.blank?

      results.select { |result| strengths_for(result).include?(normalized_strength) }
    end

    def self.normalize_component(component)
      match = COMPONENT_PATTERN.match(component)
      return unless match

      amount = BigDecimal(match[1])
      unit = UNIT_ALIASES.fetch(match[2].downcase)
      amount, unit = canonical_measurement(amount, unit)
      "#{format_amount(amount)}#{unit}"
    end
    private_class_method :normalize_component

    def self.canonical_measurement(amount, unit)
      case unit
      when 'mcg' then [amount / 1000, 'mg']
      when 'g' then [amount * 1000, 'mg']
      when 'l' then [amount * 1000, 'ml']
      else [amount, unit]
      end
    end
    private_class_method :canonical_measurement

    def self.format_amount(amount)
      amount.to_s('F').sub(/\.0+\z/, '').sub(/(\.\d*?)0+\z/, '\\1')
    end
    private_class_method :format_amount

    def self.strengths_for(result)
      searchable_values(result).flat_map do |value|
        value.to_s.scan(STRENGTH_PATTERN).filter_map { |candidate| normalize(candidate) }
      end
    end
    private_class_method :strengths_for

    def self.searchable_values(result)
      payload = result.respond_to?(:to_h) ? result.to_h : {}
      %i[display name description package_size].filter_map { |attribute| payload[attribute] }
    end
    private_class_method :searchable_values
  end
end

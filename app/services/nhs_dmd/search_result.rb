# frozen_string_literal: true

module NhsDmd
  class SearchResult
    attr_reader :barcode, :code, :display, :system, :concept_class, :match_reason, :name, :category, :package_size,
                :package_quantity, :package_unit, :directions, :warnings

    def initialize(code:, display:, system:, **attributes)
      @barcode = attributes[:barcode]
      @code = code
      @display = display
      @system = system
      @concept_class = attributes[:concept_class]
      @match_reason = attributes[:match_reason]
      @name = attributes[:name]
      @category = attributes[:category]
      @package_size = attributes[:package_size]
      @package_quantity = attributes[:package_quantity]
      @package_unit = attributes[:package_unit]
      @directions = attributes[:directions]
      @warnings = attributes[:warnings]
    end

    def concept_class_label
      case concept_class
      when 'VMP' then 'Virtual Medicinal Product'
      when 'AMP' then 'Actual Medicinal Product'
      when 'VMPP' then 'Virtual Medicinal Product Pack'
      when 'AMPP' then 'Actual Medicinal Product Pack'
      else concept_class
      end
    end

    def source_label
      case system
      when 'https://dmd.nhs.uk' then 'NHS dm+d'
      when OpenFoodFacts::Client::BASE_URL then 'Open Food Facts'
      else system
      end
    end

    def match_reason_label
      case match_reason
      when 'barcode_match' then 'Barcode match'
      end
    end

    def to_h
      core_attributes.merge(derived_attributes)
    end

    private

    def core_attributes
      {
        barcode: barcode,
        code: code,
        name: name,
        display: display,
        system: system,
        concept_class: concept_class,
        category: category,
        package_size: package_size,
        package_quantity: package_quantity,
        package_unit: package_unit,
        directions: directions,
        warnings: warnings
      }
    end

    def derived_attributes
      {
        concept_class_label: concept_class_label,
        source_label: source_label,
        match_reason: match_reason,
        match_reason_label: match_reason_label
      }
    end
  end
end

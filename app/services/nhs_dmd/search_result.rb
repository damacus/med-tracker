# frozen_string_literal: true

module NhsDmd
  class SearchResult
    attr_reader(
      :barcode,
      :code,
      :display,
      :system,
      :concept_class,
      :match_reason,
      :name,
      :category,
      :package_size,
      :package_quantity,
      :package_unit,
      :description,
      :directions,
      :warnings
    )

    def initialize(code:, display:, system:, **attributes)
      @code = code
      @display = display
      @system = system
      assign_optional_attributes(attributes)
    end

    def assign_optional_attributes(attributes)
      @barcode = attributes[:barcode]
      @concept_class = attributes[:concept_class]
      @match_reason = attributes[:match_reason]
      @name = attributes[:name]
      @category = attributes[:category]
      @package_size = attributes[:package_size]
      @package_quantity = attributes[:package_quantity]
      @package_unit = attributes[:package_unit]
      @description = attributes[:description]
      @directions = attributes[:directions]
      @warnings = attributes[:warnings]
    end

    private :assign_optional_attributes

    def concept_class_label
      case concept_class
      when "VMP"
        "Virtual Medicinal Product"
      when "AMP"
        "Actual Medicinal Product"
      when "VMPP"
        "Virtual Medicinal Product Pack"
      when "AMPP"
        "Actual Medicinal Product Pack"
      else
        concept_class
      end
    end

    def source_label
      case system
      when "https://dmd.nhs.uk"
        "NHS dm+d"
      when OpenFoodFacts::Client::BASE_URL
        "Open Food Facts"
      else
        system
      end
    end

    def match_reason_label
      case match_reason
      when "barcode_match"
        "Barcode match"
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
        description: description,
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

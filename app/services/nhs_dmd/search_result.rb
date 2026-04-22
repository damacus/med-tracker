# frozen_string_literal: true

module NhsDmd
  class SearchResult
    attr_reader :barcode, :code, :display, :system, :concept_class, :match_reason

    def initialize(code:, display:, system:, **attributes)
      @barcode = attributes[:barcode]
      @code = code
      @display = display
      @system = system
      @concept_class = attributes[:concept_class]
      @match_reason = attributes[:match_reason]
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
      {
        barcode: barcode,
        code: code,
        display: display,
        system: system,
        concept_class: concept_class,
        concept_class_label: concept_class_label,
        source_label: source_label,
        match_reason: match_reason,
        match_reason_label: match_reason_label
      }
    end
  end
end

# frozen_string_literal: true

module NhsDmd
  class SearchResult
    attr_reader :code, :display, :system, :concept_class

    def initialize(code:, display:, system:, concept_class: nil)
      @code = code
      @display = display
      @system = system
      @concept_class = concept_class
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

    def to_h
      {
        code: code,
        display: display,
        system: system,
        concept_class: concept_class,
        concept_class_label: concept_class_label
      }
    end
  end
end

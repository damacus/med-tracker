# frozen_string_literal: true

require 'uri'

module NhsDmd
  class SearchResult
    attr_reader :barcode, :code, :display, :system, :concept_class, :match_reason, :name, :category, :package_size,
                :package_quantity, :package_unit, :description, :directions, :warnings, :pil_url, :spc_url

    def initialize(code:, display:, system:, **attributes)
      @code = code
      @display = display
      @system = system
      assign_optional_attributes(attributes)
    end

    def assign_optional_attributes(attributes)
      assign_source_attributes(attributes)
      assign_package_attributes(attributes)
      assign_guidance_attributes(attributes)
    end

    private :assign_optional_attributes

    def assign_source_attributes(attributes)
      @barcode = attributes[:barcode]
      @concept_class = attributes[:concept_class]
      @match_reason = attributes[:match_reason]
      @name = attributes[:name]
      @category = attributes[:category]
    end

    def assign_package_attributes(attributes)
      @package_size = attributes[:package_size]
      @package_quantity = attributes[:package_quantity]
      @package_unit = attributes[:package_unit]
    end

    def assign_guidance_attributes(attributes)
      @description = attributes[:description]
      @directions = attributes[:directions]
      @warnings = attributes[:warnings]
      @pil_url = safe_https_url(attributes[:pil_url])
      @spc_url = safe_https_url(attributes[:spc_url])
    end

    private :assign_source_attributes, :assign_package_attributes, :assign_guidance_attributes

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
      identity_attributes.merge(package_attributes, guidance_attributes)
    end

    def identity_attributes
      {
        barcode: barcode,
        code: code,
        name: name,
        description: description,
        display: display,
        system: system,
        concept_class: concept_class,
        category: category
      }
    end

    def package_attributes
      {
        package_size: package_size,
        package_quantity: package_quantity,
        package_unit: package_unit
      }
    end

    def guidance_attributes
      {
        directions: directions,
        warnings: warnings,
        pil_url: pil_url,
        spc_url: spc_url
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

    def safe_https_url(url)
      normalized_url = url.to_s.strip
      return if normalized_url.blank?

      uri = URI.parse(normalized_url)
      return unless uri.is_a?(URI::HTTPS) && uri.host.present?

      uri.to_s
    rescue URI::InvalidURIError
      nil
    end
  end
end

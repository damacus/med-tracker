# frozen_string_literal: true

module NhsDmd
  class DosageFormFilter
    def self.normalize_text(value)
      value.to_s.downcase.gsub(/[^a-z0-9]+/, '_').gsub(/\A_+|_+\z/, '')
    end
    private_class_method :normalize_text

    OPTIONS = {
      'tablet' => 'Tablets',
      'capsule' => 'Capsules',
      'liquid' => 'Liquids',
      'cream' => 'Creams and ointments',
      'inhaler' => 'Inhalers',
      'injection' => 'Injections',
      'patch' => 'Patches',
      'drops' => 'Drops',
      'spray' => 'Sprays',
      'powder' => 'Powders and sachets'
    }.freeze

    TERMS = {
      'tablet' => %w[tablet tablets caplet caplets],
      'capsule' => %w[capsule capsules],
      'liquid' => %w[liquid solution suspension syrup oral_solution oral_suspension ml],
      'cream' => %w[cream ointment gel],
      'inhaler' => %w[inhaler inhalation inhalation_powder nebuliser nebulizer],
      'injection' => %w[injection injectable syringe ampoule vial],
      'patch' => %w[patch patches],
      'drops' => %w[drops eye_drops ear_drops],
      'spray' => %w[spray sprays],
      'powder' => %w[powder powders sachet sachets granules]
    }.freeze

    ALIASES = TERMS.each_with_object({}) do |(form, terms), aliases|
      aliases[form] = form
      terms.each { |term| aliases[normalize_text(term)] = form }
    end.freeze

    def self.options
      OPTIONS
    end

    def self.normalize(value)
      ALIASES[normalize_text(value)]
    end

    def self.filter(results, form)
      normalized_form = normalize(form)
      return results if normalized_form.blank?

      results.select { |result| matches?(result, normalized_form) }
    end

    def self.matches?(result, form)
      searchable_text = normalize_text(searchable_values(result).join(' '))
      TERMS.fetch(form, []).any? { |term| searchable_text.include?(normalize_text(term)) }
    end

    def self.searchable_values(result)
      payload = result.respond_to?(:to_h) ? result.to_h : {}
      [
        payload[:package_unit],
        payload[:display],
        payload[:name],
        payload[:description],
        result.respond_to?(:package_unit) ? result.package_unit : nil,
        result.respond_to?(:display) ? result.display : nil,
        result.respond_to?(:name) ? result.name : nil
      ].compact
    end

    private_class_method :matches?, :searchable_values
  end
end

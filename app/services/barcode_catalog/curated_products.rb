# frozen_string_literal: true

require 'yaml'

module BarcodeCatalog
  class CuratedProducts
    Dose = Data.define(
      :amount,
      :unit,
      :frequency,
      :description,
      :default_for_adults,
      :default_for_children,
      :default_max_daily_doses,
      :default_min_hours_between_doses,
      :default_dose_cycle,
      :current_supply,
      :reorder_threshold
    ) do
      def to_dosage_attributes
        {
          amount: amount,
          unit: unit,
          frequency: frequency,
          description: description,
          default_for_adults: default_for_adults,
          default_for_children: default_for_children,
          default_max_daily_doses: default_max_daily_doses,
          default_min_hours_between_doses: default_min_hours_between_doses,
          default_dose_cycle: default_dose_cycle,
          current_supply: current_supply,
          reorder_threshold: reorder_threshold
        }
      end
    end

    Product = Data.define(
      :gtin,
      :code,
      :display,
      :system,
      :concept_class,
      :category,
      :description,
      :warnings,
      :suggested_doses
    ) do
      def lookup_attributes
        {
          barcode: gtin,
          code: code,
          display: display,
          system: system,
          concept_class: concept_class,
          source: 'curated'
        }
      end

      def dosage_attributes
        suggested_doses.map(&:to_dosage_attributes)
      end

      def medication_attributes
        {
          category: category,
          description: description,
          warnings: warnings
        }.compact
      end
    end

    class << self
      def lookup_gtin(gtin)
        normalized_gtin = NhsDmdBarcode.normalize_gtin(gtin)
        return if normalized_gtin.blank?

        products.find { |product| product.gtin == normalized_gtin }
      end

      def find(barcode: nil, code: nil, name: nil)
        normalized_barcode = NhsDmdBarcode.normalize_gtin(barcode)

        products.find do |product|
          (normalized_barcode.present? && product.gtin == normalized_barcode) ||
            (code.present? && product.code == code) ||
            (name.present? && product.display == name)
        end
      end

      private

      def products
        @products ||= begin
          raw_products = YAML.load_file(Rails.root.join('config/nhs_dmd_curated_products.yml'))
          Array(raw_products['products']).map { |entry| build_product(entry) }
        end
      end

      def build_product(entry)
        Product.new(
          gtin: entry['gtin'],
          code: entry['code'],
          display: entry.fetch('display'),
          system: entry['system'],
          concept_class: entry['concept_class'],
          category: entry['category'],
          description: entry['description'],
          warnings: entry['warnings'],
          suggested_doses: Array(entry['suggested_doses']).map { |dose| build_dose(dose) }
        )
      end

      def build_dose(entry)
        Dose.new(
          amount: entry.fetch('amount'),
          unit: entry.fetch('unit'),
          frequency: entry.fetch('frequency'),
          description: entry['description'],
          default_for_adults: entry.fetch('default_for_adults', false),
          default_for_children: entry.fetch('default_for_children', false),
          default_max_daily_doses: entry.fetch('default_max_daily_doses'),
          default_min_hours_between_doses: entry.fetch('default_min_hours_between_doses'),
          default_dose_cycle: entry.fetch('default_dose_cycle'),
          current_supply: entry['current_supply'],
          reorder_threshold: entry['reorder_threshold']
        )
      end
    end
  end
end

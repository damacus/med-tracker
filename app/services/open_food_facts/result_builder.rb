# frozen_string_literal: true

module OpenFoodFacts
  module ResultBuilder
    SUPPLEMENT_CATEGORY_KEYWORDS = [
      'supplement',
      'supplements',
      'food supplements',
      'vitamin',
      'vitamins',
      'multivitamin',
      'multivitamins'
    ].freeze

    module_function

    def search_result_from_product(product)
      payload = result_payload(product)
      return nil unless payload

      build_result(payload)
    end

    def result_payload(product)
      return nil unless product

      payload = normalized_payload(product)
      return nil if product_name(payload).blank? || !supplement_product?(payload)

      payload
    end

    def build_result(payload)
      {
        code: nil,
        barcode: normalized_barcode(payload),
        display: display_for(payload),
        system: Client::BASE_URL,
        concept_class: 'Supplement',
        source: 'open_food_facts'
      }
    end

    def normalized_payload(product)
      base_payload = product.fetch('product', product)
      base_payload.merge('code' => product['code'] || base_payload['code'])
    end

    def display_for(payload)
      [
        product_name(payload),
        brand_segment(payload),
        quantity_segment(payload)
      ].compact_blank.join(' ')
    end

    def supplement_product?(payload)
      Array(payload['categories_tags_en']).any? do |category|
        normalized = category.to_s.downcase
        SUPPLEMENT_CATEGORY_KEYWORDS.any? { |keyword| normalized.include?(keyword) }
      end
    end

    def product_name(payload)
      payload['product_name'].to_s.strip
    end

    def brand_segment(payload)
      brands = payload['brands'].to_s.strip
      return nil if brands.blank?

      "(#{brands})"
    end

    def quantity_segment(payload)
      payload['quantity'].to_s.strip.presence
    end

    def normalized_barcode(payload)
      code = payload['code'].to_s
      return nil if code.blank?

      BarcodeCatalogEntry.normalize_gtin(code)
    end
  end
end

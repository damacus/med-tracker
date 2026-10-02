# frozen_string_literal: true

module OpenFoodFacts
  module ResultBuilder
    PACK_UNIT_MAP = ({
      'tablet' => %w[tablet tablets],
      'capsule' => %w[capsule capsules],
      'sachet' => %w[sachet sachets],
      'spray' => %w[spray sprays],
      'drop' => %w[drop drops],
      'pad' => %w[pad pads],
      'ml' => %w[ml millilitre millilitres milliliter milliliters],
      'g' => %w[g gram grams]
    }.each_with_object({}) do |(normalized, units), map|
      units.each { |unit| map[unit] = normalized }
    end).freeze
    SUPPLEMENT_CATEGORY_KEYWORDS = ['supplement', 'supplements', 'food supplements', 'vitamin', 'vitamins',
                                    'multivitamin', 'multivitamins'].freeze

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
        name: product_name(payload),
        description: generic_name(payload),
        display: display_for(payload),
        system: Client::BASE_URL,
        category: 'Supplement',
        package_size: quantity_segment(payload),
        package_quantity: package_quantity(payload),
        package_unit: package_unit(payload),
        concept_class: 'Supplement',
        source: 'open_food_facts'
      }
    end

    def normalized_payload(product)
      base_payload = product.fetch('product', product)
      base_payload.merge('code' => product['code'] || base_payload['code'])
    end

    def display_for(payload)
      [product_name(payload), brand_segment(payload), quantity_segment(payload)].compact_blank.join(' ')
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

    def generic_name(payload)
      payload['generic_name'].to_s.strip.presence
    end

    def brand_segment(payload)
      brands = payload['brands'].to_s.strip
      return nil if brands.blank?

      "(#{brands})"
    end

    def quantity_segment(payload)
      payload['quantity'].to_s.strip.presence
    end

    def quantity_match(payload)
      quantity_segment(payload)&.match(/\A\s*(\d+(?:[.,]\d+)?)\s*([[:alpha:]]+)?\b/i)
    end

    def package_quantity(payload)
      normalize_quantity(quantity_match(payload)&.captures&.first)
    end

    def package_unit(payload)
      raw_unit = quantity_match(payload)&.captures&.second
      return nil if raw_unit.blank?

      PACK_UNIT_MAP.fetch(raw_unit.downcase, nil)
    end

    def normalized_barcode(payload)
      code = payload['code'].to_s
      return nil if code.blank?

      BarcodeCatalogEntry.normalize_gtin(code)
    end

    def normalize_quantity(raw_quantity)
      return nil if raw_quantity.blank?

      numeric = raw_quantity.tr(',', '.')
      return numeric.to_i if numeric.match?(/\A\d+\z/)

      numeric.to_f
    end
  end
end

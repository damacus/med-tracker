# frozen_string_literal: true

module OpenProductsFacts
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
    MEDICINE_CATEGORY_KEYWORDS = %w[
      medicine medicines medication medications pharmaceutical pharmaceuticals pharmacy pharmacies drug drugs
      analgesic analgesics painkiller painkillers paracetamol acetaminophen ibuprofen aspirin
    ].freeze

    module_function

    def catalog_entry_from_product(barcode, product)
      payload = normalized_payload(product)
      return nil if product_name(payload).blank?
      return nil unless medicine_product?(payload)

      {
        gtin: BarcodeCatalogEntry.normalize_gtin(barcode),
        display: display_for(payload),
        source: 'open_products_facts',
        system: Client::BASE_URL,
        concept_class: 'OTC Medicine'
      }
    end

    def normalized_payload(product)
      base_payload = product.fetch('product', product)
      base_payload.merge('code' => product['code'] || base_payload['code'])
    end

    def display_for(payload)
      [product_name(payload), brand_segment(payload), quantity_segment(payload)].compact_blank.join(' ')
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

    def medicine_product?(payload)
      category_tokens(payload).any? do |category|
        normalized = category.to_s.downcase
        MEDICINE_CATEGORY_KEYWORDS.any? { |keyword| normalized.match?(/\b#{Regexp.escape(keyword)}\b/) }
      end
    end

    def category_tokens(payload)
      Array(payload['categories_tags_en']) + Array(payload['categories_tags'])
    end
  end
end

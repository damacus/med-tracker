# frozen_string_literal: true

module OpenFoodFacts
  class BarcodeLookup
    def initialize(client: Client.new)
      @client = client
    end

    def lookup(barcode)
      product = @client.product(barcode)
      ResultBuilder.search_result_from_product(product)
    rescue Client::ApiError => e
      Rails.logger.warn("OpenFoodFacts::BarcodeLookup failed: #{e.message}")
      nil
    rescue StandardError => e
      Rails.logger.error("OpenFoodFacts::BarcodeLookup crashed: #{e.class}: #{e.message}")
      nil
    end
  end
end

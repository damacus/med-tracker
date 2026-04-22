# frozen_string_literal: true

module OpenFoodFacts
  class Search
    DEFAULT_PAGE_SIZE = 10

    def initialize(client: Client.new)
      @client = client
    end

    def search(query, page_size: DEFAULT_PAGE_SIZE)
      @client.search_products(query, page_size: page_size).filter_map do |product|
        ResultBuilder.search_result_from_product(product)
      end
    rescue Client::ApiError => e
      Rails.logger.warn("OpenFoodFacts::Search failed: #{e.message}")
      []
    rescue StandardError => e
      Rails.logger.error("OpenFoodFacts::Search crashed: #{e.class}: #{e.message}")
      []
    end
  end
end

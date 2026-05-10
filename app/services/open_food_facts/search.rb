# frozen_string_literal: true

module OpenFoodFacts
  class Search
    DEFAULT_PAGE_SIZE = 10

    def initialize(client: Client.new, audit_logger: ExternalLookup::AuditLogger.new)
      @client = client
      @audit_logger = audit_logger
    end

    def search(query, page_size: DEFAULT_PAGE_SIZE)
      results = @client.search_products(query, page_size: page_size).filter_map do |product|
        ResultBuilder.search_result_from_product(product)
      end

      audit(query, results.any? ? "success" : "not_found", results.size)
      results
    rescue Client::ApiError => e
      Rails.logger.warn("OpenFoodFacts::Search failed: #{e.message}")
      audit(query, "error")
      []
    rescue StandardError => e
      Rails.logger.error("OpenFoodFacts::Search crashed: #{e.class}: #{e.message}")
      audit(query, "error")
      []
    end

    private

    def audit(query, status, count = 0)
      @audit_logger.record(
        source: "open_food_facts",
        event: "search",
        query: query,
        result_status: status,
        result_count: count
      )
    end
  end
end

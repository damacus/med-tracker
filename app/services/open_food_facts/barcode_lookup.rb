# frozen_string_literal: true

module OpenFoodFacts
  class BarcodeLookup
    def initialize(client: Client.new, audit_logger: ExternalLookup::AuditLogger.new)
      @client = client
      @audit_logger = audit_logger
    end

    def lookup(barcode)
      product = @client.product(barcode)
      result = ResultBuilder.search_result_from_product(product)
      audit(barcode, result ? 'success' : 'not_found', result ? 1 : 0)
      result
    rescue Client::ApiError => e
      Rails.logger.warn("OpenFoodFacts::BarcodeLookup failed: #{e.message}")
      audit(barcode, 'error')
      nil
    rescue StandardError => e
      Rails.logger.error("OpenFoodFacts::BarcodeLookup crashed: #{e.class}: #{e.message}")
      audit(barcode, 'error')
      nil
    end

    private

    def audit(barcode, status, count = 0)
      @audit_logger.record(source: 'open_food_facts', event: 'barcode_lookup',
                           query: barcode, result_status: status, result_count: count)
    end
  end
end

# frozen_string_literal: true

module NhsDmd
  class Search
    Result = Struct.new(:results, :error, :resolved_query, :barcode, keyword_init: true) do
      def success?
        error.nil?
      end
    end

    def initialize(client: Client.new, barcode_lookup: BarcodeCatalog::Lookup.new)
      @client = client
      @barcode_lookup = barcode_lookup
    end

    def call(query)
      return Result.new(results: [], error: nil) if query.blank?

      barcode_match = @barcode_lookup.lookup(query)
      return barcode_result(query, barcode_match) if barcode_match

      return not_configured_result unless @client.configured?

      Result.new(results: search_results(query), error: nil)
    rescue Client::ApiError => e
      failed_result(e.message)
    rescue StandardError => e
      failed_result('unexpected_error', e)
    end

    private

    def not_configured_result
      Result.new(results: [], error: 'not_configured')
    end

    def search_results(query)
      build_results(@client.search(query))
    end

    def failed_result(message, exception = nil)
      log_failure(message, exception)
      Result.new(results: [], error: message)
    end

    def log_failure(message, exception)
      if exception
        Rails.logger.error("NhsDmd::Search crashed: #{exception.class}: #{exception.message}")
      else
        Rails.logger.error("NhsDmd::Search failed: #{message}")
      end
    end

    def build_result(item)
      SearchResult.new(
        code: item[:code],
        display: item[:display],
        system: item[:system],
        concept_class: item[:concept_class]
      )
    end

    def barcode_result(query, barcode_match)
      translated_query = barcode_match[:display]

      Result.new(
        results: build_results(barcode_items(translated_query, barcode_match)),
        error: nil,
        resolved_query: translated_query,
        barcode: NhsDmdBarcode.normalize_gtin(query)
      )
    end

    def build_results(items)
      items.uniq { |item| item[:code] }.map { |item| build_result(item) }
    end

    def barcode_items(translated_query, barcode_match)
      items = []
      items << barcode_match if barcode_match[:code].present?
      items.concat(@client.search(translated_query)) if @client.configured?
      items
    rescue Client::ApiError => e
      log_failure(e.message)
      [barcode_match]
    rescue StandardError => e
      log_failure('unexpected_error', e)
      [barcode_match]
    end
  end
end

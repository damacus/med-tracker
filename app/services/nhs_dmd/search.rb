# frozen_string_literal: true

module NhsDmd
  class Search
    Result = Struct.new(:results, :error, keyword_init: true) do
      def success?
        error.nil?
      end
    end

    def initialize(client: Client.new, barcode_lookup: BarcodeLookup.new)
      @client = client
      @barcode_lookup = barcode_lookup
    end

    def call(query)
      return Result.new(results: [], error: nil) if query.blank?

      barcode_match = @barcode_lookup.lookup(query)
      return Result.new(results: [build_result(barcode_match)], error: nil) if barcode_match

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
      @client.search(query).map { |item| build_result(item) }
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
  end
end

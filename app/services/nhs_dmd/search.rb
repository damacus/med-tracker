# frozen_string_literal: true

module NhsDmd
  class Search
    Result = Struct.new(:results, :error, keyword_init: true) do
      def success?
        error.nil?
      end
    end

    def initialize(client: Client.new)
      @client = client
    end

    def call(query)
      return not_configured_result unless @client.configured?
      return Result.new(results: [], error: nil) if query.blank?

      raw = @client.search(query)
      results = raw.map { |item| build_result(item) }
      Result.new(results: results, error: nil)
    rescue Client::ApiError => e
      Rails.logger.error("NhsDmd::Search failed: #{e.message}")
      Result.new(results: [], error: e.message)
    end

    private

    def not_configured_result
      Result.new(results: [], error: 'not_configured')
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

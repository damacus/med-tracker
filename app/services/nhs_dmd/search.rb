# frozen_string_literal: true

module NhsDmd
  class Search
    Result = Struct.new(:results, :error, :resolved_query, :barcode, keyword_init: true) do
      def success?
        error.nil?
      end
    end

    def initialize(client: Client.new, barcode_lookup: BarcodeCatalog::Lookup.new,
                   open_food_facts_lookup: OpenFoodFacts::BarcodeLookup.new,
                   open_food_facts_search: OpenFoodFacts::Search.new)
      @client = client
      @barcode_lookup = barcode_lookup
      @open_food_facts_lookup = open_food_facts_lookup
      @open_food_facts_search = open_food_facts_search
    end

    def call(query)
      return empty_result if query.blank?

      local_barcode_result(query) || remote_result(query)
    rescue Client::ApiError => e
      failed_result(e.message)
    rescue StandardError => e
      failed_result('unexpected_error', e)
    end

    private

    def not_configured_result
      Result.new(results: [], error: 'not_configured')
    end

    def empty_result
      Result.new(results: [], error: nil)
    end

    def local_barcode_result(query)
      barcode_match = @barcode_lookup.lookup(query)
      return nil unless barcode_match

      barcode_result(query, barcode_match)
    end

    def remote_result(query)
      barcode_results = configured_barcode_results(query)
      return barcode_results if barcode_results

      off_match = open_food_facts_result_for(query)
      return barcode_result(query, off_match) if off_match

      return unconfigured_result(query) unless @client.configured?

      configured_result(query)
    end

    def unconfigured_result(query)
      supplement_results = likely_supplement_text_query?(query) ? text_supplement_items(query) : []
      return Result.new(results: build_results(supplement_results), error: nil) if supplement_results.any?

      not_configured_result
    end

    def configured_result(query)
      nhs_results = nhs_items(query)
      supplement_results = supplement_query_needed?(query, nhs_results) ? text_supplement_items(query) : []
      results = merge_items(supplement_results, nhs_results)
      return Result.new(results: build_results(results), error: nil) if results.any?

      empty_result
    end

    def nhs_items(query)
      @client.search(query)
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
        **search_result_attributes(item)
      )
    end

    def search_result_attributes(item)
      {
        barcode: item[:barcode],
        name: item[:name],
        description: item[:description],
        concept_class: item[:concept_class],
        category: item[:category],
        package_size: item[:package_size],
        package_quantity: item[:package_quantity],
        package_unit: item[:package_unit],
        directions: item[:directions],
        warnings: item[:warnings],
        match_reason: item[:match_reason]
      }
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
      dedupe_items(items).map { |item| build_result(item) }
    end

    def barcode_items(translated_query, barcode_match)
      authoritative_items = authoritative_barcode_items(translated_query, barcode_match)
      return authoritative_items if authoritative_items

      fallback_barcode_items(translated_query, barcode_match)
    rescue Client::ApiError => e
      log_failure(e.message)
      [barcode_match]
    rescue StandardError => e
      log_failure('unexpected_error', e)
      [barcode_match]
    end

    def authoritative_barcode_items(translated_query, barcode_match)
      return supplement_barcode_items(translated_query, barcode_match) if supplement_item?(barcode_match)
      return [barcode_match_item(translated_query, barcode_match)] if barcode_match[:code].present?
      return [barcode_match_item(translated_query, barcode_match)] if curated_barcode_match?(barcode_match)

      nil
    end

    def fallback_barcode_items(translated_query, barcode_match)
      items = @client.configured? ? @client.search(translated_query) : []
      items = [barcode_match] if items.empty? && barcode_match[:display].present?
      items
    end

    def open_food_facts_result_for(query)
      return nil unless BarcodeLookup.barcode_query?(query)

      @open_food_facts_lookup.lookup(query)
    end

    def supplement_barcode_items(translated_query, barcode_match)
      merge_items([barcode_match_item(translated_query, barcode_match)], text_supplement_items(translated_query))
    end

    def text_supplement_items(query)
      return [] if BarcodeLookup.barcode_query?(query)

      @open_food_facts_search.search(query)
    end

    def supplement_query_needed?(query, nhs_results)
      return false if numeric_query?(query)
      return likely_supplement_text_query?(query) if nhs_results.empty?

      low_nhs_relevance?(query, nhs_results)
    end

    def supplement_item?(item)
      item[:source] == 'open_food_facts' ||
        item[:system] == OpenFoodFacts::Client::BASE_URL ||
        item[:concept_class] == 'Supplement'
    end

    def curated_barcode_match?(item)
      item[:source] == 'curated'
    end

    def likely_supplement_text_query?(query)
      query_words(query).intersect?(OpenFoodFacts::ResultBuilder::SUPPLEMENT_CATEGORY_KEYWORDS)
    end

    def low_nhs_relevance?(query, nhs_results)
      query_tokens = query_words(query)
      return false if query_tokens.empty?

      nhs_results.none? do |item|
        query_tokens.intersect?(query_words(item[:display]))
      end
    end

    def numeric_query?(query)
      query.to_s.strip.match?(/\A\d+\z/)
    end

    def query_words(text)
      text.to_s.downcase.scan(/[[:alnum:]]+/).reject { |word| word.length < 3 }
    end

    def merge_items(*groups)
      dedupe_items(groups.flatten.compact)
    end

    def configured_barcode_results(query)
      return nil unless @client.configured? && BarcodeLookup.barcode_query?(query)

      results = nhs_items(query)
      return nil if results.empty?

      Result.new(results: build_results(results), error: nil, barcode: NhsDmdBarcode.normalize_gtin(query))
    end

    def dedupe_items(items)
      items.uniq do |item|
        [item[:system], item[:code], item[:barcode], item[:display]]
      end
    end

    def barcode_match_item(translated_query, barcode_match)
      exact = exact_nhs_match(translated_query, barcode_match)
      annotate_barcode_match(exact || barcode_match)
    end

    def exact_nhs_match(translated_query, barcode_match)
      return nil unless @client.configured?
      return nil if supplement_item?(barcode_match)
      return nil if curated_barcode_match?(barcode_match)

      nhs_items(translated_query).find do |item|
        item[:code] == barcode_match[:code] && item[:system] == barcode_match[:system]
      end
    rescue Client::ApiError => e
      log_failure(e.message, e)
      nil
    rescue StandardError => e
      log_failure('unexpected_error', e)
      nil
    end

    def annotate_barcode_match(item)
      item.merge(match_reason: 'barcode_match')
    end
  end
end

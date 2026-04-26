# frozen_string_literal: true

require 'uri'

module NhsWebsiteContent
  class MedicineGuidanceLookup
    Section = Data.define(:title, :text)
    Result = Data.define(:title, :description, :webpage, :last_reviewed_on, :sections, :author_name, :author_url,
                         :author_logo)

    FORM_WORDS = %w[
      tablet tablets capsule capsules sachet sachets oral solution suspension liquid cream ointment gel spray sprays
      drop drops powder granules injection injections patch patches inhaler inhalers lozenge lozenges
      suppository suppositories
    ].freeze
    STRENGTH_PATTERN = %r{
      \b
      \d+(?:\.\d+)?
      \s*
      (?:mg|ml|mcg|g|gram|grams|microgram|micrograms|units|iu|%)
      (?:\s*/\s*\d+(?:\.\d+)?\s*(?:ml|g))?
      \b
    }ix
    SECTION_PRIORITY = {
      'OverviewHealthAspect' => 0,
      'UsageOrScheduleHealthAspect' => 1,
      'SideEffectsHealthAspect' => 2,
      'InteractionsHealthAspect' => 3,
      'WarningHealthAspect' => 4,
      'SuitabilityHealthAspect' => 5
    }.freeze

    def initialize(client: Client.new)
      @client = client
    end

    def call(name)
      return nil if name.blank?
      return nil unless @client.configured?

      page = resolve_page(name)
      return nil unless page

      build_result(@client.get_medicine(slug: slug_from(page['url']), modules: true))
    rescue Client::ApiError => e
      Rails.logger.error("NhsWebsiteContent::MedicineGuidanceLookup failed: #{e.message}")
      nil
    rescue StandardError => e
      Rails.logger.error("NhsWebsiteContent::MedicineGuidanceLookup crashed: #{e.class}: #{e.message}")
      nil
    end

    private

    def resolve_page(name)
      ranked = ranked_matches(name)
      return nil if ranked.empty?
      return nil if ambiguous?(ranked)

      ranked.first.first
    end

    def ambiguous?(ranked)
      return false if ranked.length < 2

      ranked.first.last - ranked.second.last < 10
    end

    def list_pages_for(category)
      page = '1'
      results = []

      loop do
        response = @client.list_medicines(category: category, page: page)
        results.concat(Array(response['significantLink']))
        page = next_page_number(response)
        break if page.blank?
      end

      results
    end

    def next_page_number(response)
      link = Array(response['relatedLink']).find do |related_link|
        related_link['name'] == 'Next Page' || related_link['linkRelationship'] == 'Next'
      end
      return nil if link.blank?

      URI.decode_www_form(URI.parse(link['url']).query.to_s).to_h['page']
    rescue URI::InvalidURIError
      nil
    end

    def score(page, query)
      score_for(
        page_name: normalized(page['name']),
        description: normalized(page['description']),
        query: query,
        query_tokens: query.split
      )
    end

    def score_for(page_name:, description:, query:, query_tokens:)
      return 100 if exact_match?(page_name, query)
      return 85 if prefix_match?(page_name, query)
      return 70 if description_match?(description, query)
      return 60 if token_subset?(query_tokens, page_name)
      return 50 if token_subset?(query_tokens, description)
      return 40 if token_prefix_match?(query_tokens, page_name)

      0
    end

    def candidate_queries(name)
      stripped = name.to_s.gsub(/\([^)]*\)/, ' ')

      [
        normalized(stripped),
        normalized(stripped.gsub(STRENGTH_PATTERN, ' ')),
        normalized(remove_form_words(stripped.gsub(STRENGTH_PATTERN, ' ')))
      ].compact_blank.uniq
    end

    def remove_form_words(text)
      FORM_WORDS.reduce(text.to_s) do |memo, word|
        memo.gsub(/\b#{Regexp.escape(word)}\b/i, ' ')
      end
    end

    def normalized(text)
      text.to_s.downcase
          .gsub(/[^a-z0-9\s]/, ' ')
          .gsub(/\b(?:ltd|limited|plc|inc|uk)\b/, ' ')
          .squish
    end

    def slug_from(url)
      URI.parse(url).path.split('/').compact_blank.last
    end

    def build_result(payload)
      Result.new(
        title: payload['name'],
        description: payload['description'],
        webpage: payload['webpage'],
        last_reviewed_on: last_reviewed_on(payload),
        sections: build_sections(payload),
        author_name: payload.dig('author', 'name'),
        author_url: payload.dig('author', 'url'),
        author_logo: payload.dig('author', 'logo')
      )
    end

    def last_reviewed_on(payload)
      last_reviewed = Array(payload.dig('mainEntityOfPage', 'lastReviewed')).last
      return nil if last_reviewed.blank?

      Date.parse(last_reviewed)
    rescue Date::Error
      nil
    end

    def build_sections(payload)
      sections = Array(payload['hasPart']).filter_map do |part|
        text = stripped_text(part['text'])
        next if text.blank?

        Section.new(title: section_title(part), text: text)
      end

      sections.sort_by { |section| SECTION_PRIORITY.fetch(section_priority_key(section.title), 99) }
              .first(3)
    end

    def section_title(part)
      part['headline'].presence || part['name'].presence || humanized_health_aspect(part['healthAspect'])
    end

    def section_priority_key(title)
      SECTION_PRIORITY.keys.find { |key| humanized_health_aspect(key) == title }
    end

    def humanized_health_aspect(value)
      value.to_s.delete_suffix('HealthAspect').gsub(/([a-z])([A-Z])/, '\1 \2').presence
    end

    def stripped_text(text)
      ActionController::Base.helpers.strip_tags(text.to_s).squish
    end

    def exact_match?(page_name, query)
      page_name == query
    end

    def prefix_match?(page_name, query)
      page_name.start_with?(query) || query.start_with?(page_name)
    end

    def description_match?(description, query)
      description.include?(query)
    end

    def token_subset?(query_tokens, haystack)
      (query_tokens - haystack.split).empty?
    end

    def token_prefix_match?(query_tokens, page_name)
      query_tokens.first.present? && page_name.split.include?(query_tokens.first)
    end

    def ranked_matches(name)
      deduplicated_ranked_matches(scored_ranked_matches(name)).sort_by { |(_page, match_score)| -match_score }
    end

    def scored_pages_for(query)
      return [] if query.blank?

      list_pages_for(query.first.upcase).map { |page| [page, score(page, query)] }
    end

    def page_key(page)
      page['url'].presence || normalized(page['name'])
    end

    def scored_ranked_matches(name)
      candidate_queries(name)
        .flat_map { |query| scored_pages_for(query) }
        .select { |(_page, match_score)| match_score >= 40 }
    end

    def deduplicated_ranked_matches(scored_matches)
      ranked_pages = {}

      scored_matches.each do |page, match_score|
        key = page_key(page)
        current = ranked_pages[key]
        ranked_pages[key] = [page, match_score] if current.blank? || match_score > current.last
      end

      ranked_pages.values
    end
  end
end

# frozen_string_literal: true

module AiMedication
  module Tools
    class SearchMedicationSources < (defined?(RubyLLM::Tool) ? RubyLLM::Tool : Object)
      description "Searches configured trusted medication guidance source seeds" if respond_to?(:description)

      if respond_to?(:params)
        params do
          string :query, description: "Medication source search query"
        end
      end

      def initialize(allowlist: TrustedSourceAllowlist.new)
        super()
        @allowlist = allowlist
      end

      def execute(query:)
        tokens = query_tokens(query)
        @allowlist.seed_urls.filter_map do |source|
          next unless source_allowed?(source)
          next unless source_matches?(source, tokens)

          {
            url: source.fetch("url"),
            title: source.fetch("title"),
            matched_keywords: matching_keywords(source, tokens)
          }
        end
      end

      private

      def source_allowed?(source)
        @allowlist.allowed?(source.fetch("url"))
      end

      def source_matches?(source, tokens)
        matching_keywords(source, tokens).any?
      end

      def matching_keywords(source, tokens)
        Array(source.fetch("keywords", [])).select do |keyword|
          keyword_tokens = query_tokens(keyword)
          keyword_tokens.any? { |token| tokens.include?(token) }
        end
      end

      def query_tokens(value)
        value.to_s.downcase.scan(/[[:alnum:]]+/).reject { |token| token.length < 3 }
      end
    end
  end
end

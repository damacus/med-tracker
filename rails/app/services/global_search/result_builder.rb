# frozen_string_literal: true

module GlobalSearch
  class ResultBuilder
    attr_reader :query

    def initialize(query:)
      @query = query.to_s.strip
    end

    def build(type:, title:, subtitle:, path:, secondary_values: [])
      Result.new(
        type: type,
        title: title,
        subtitle: subtitle,
        path: path,
        score: score_for(title, secondary_values)
      )
    end

    def rescore(result, secondary_values: [])
      build(
        type: result.type,
        title: result.title,
        subtitle: result.subtitle,
        path: result.path,
        secondary_values: secondary_values
      )
    end

    private

    def score_for(title, secondary_values)
      normalized_title = normalize(title)
      return 100 if normalized_title == normalized_query
      return 80 if normalized_title.start_with?(normalized_query)
      return 60 if normalized_title.include?(normalized_query)
      return 40 if secondary_values.any? { |value| normalize(value).include?(normalized_query) }

      0
    end

    def normalize(value)
      value.to_s.strip.downcase
    end

    def normalized_query
      @normalized_query ||= normalize(query)
    end
  end
end

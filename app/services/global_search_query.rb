# frozen_string_literal: true

class GlobalSearchQuery
  Result = GlobalSearch::Result

  TYPE_ORDER = {
    "command" => 0,
    "person" => 1,
    "medication" => 2,
    "person_medication" => 3,
    "schedule" => 4,
    "location" => 5
  }.freeze

  RECORD_QUERIES = [
    GlobalSearch::PeopleResultsQuery,
    GlobalSearch::MedicationsResultsQuery,
    GlobalSearch::LocationsResultsQuery,
    GlobalSearch::SchedulesResultsQuery,
    GlobalSearch::PersonMedicationsResultsQuery
  ].freeze

  attr_reader :user, :query, :limit

  def initialize(user:, query:, limit: 12)
    @user = user
    @query = query.to_s.strip
    @limit = limit.to_i.positive? ? limit.to_i : 12
  end

  def call
    return [] unless user
    return command_results.first(limit) if query.blank?

    results = matching_command_results
    results.concat(record_results) if query.length >= 2
    sort_results(results)
  end

  private

  def record_results
    RECORD_QUERIES.flat_map { |query_class| record_results_for(query_class) }
  end

  def command_results
    GlobalSearchCommandsQuery.new(user: user).call
  end

  def matching_command_results
    command_results.filter_map do |result|
      rescored_result = result_builder.rescore(result, secondary_values: [result.subtitle])
      next unless rescored_result.score.positive?

      rescored_result
    end
  end

  def record_results_for(query_class)
    query_class.new(user: user, query: query, limit: limit, builder: result_builder).call
  end

  def result_builder
    @result_builder ||= GlobalSearch::ResultBuilder.new(query: query)
  end

  def sort_results(results)
    results
      .select { |result| result.score.positive? }
      .sort_by { |result| [-result.score, TYPE_ORDER.fetch(result.type), result.title.downcase] }
      .first(limit)
  end
end

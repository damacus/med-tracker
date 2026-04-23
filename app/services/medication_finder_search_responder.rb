# frozen_string_literal: true

class MedicationFinderSearchResponder
  Result = Data.define(:body, :status)

  def initialize(search: NhsDmd::Search.new)
    @search = search
  end

  def call(query:)
    normalized_query = query.to_s.strip
    return Result.new(body: { results: [] }, status: :ok) if normalized_query.blank?

    result = @search.call(normalized_query)
    return unavailable_response unless result&.success?

    successful_response(query: normalized_query, result: result)
  rescue StandardError => e
    Rails.logger.error("Medication finder search failed: #{e.class}: #{e.message}")
    unavailable_response
  end

  private

  def successful_response(query:, result:)
    Result.new(
      body: {
        results: result.results.map(&:to_h),
        query: result.resolved_query.presence || query,
        barcode: result.barcode
      },
      status: :ok
    )
  end

  def unavailable_response
    Result.new(
      body: { results: [], error: 'Medication search is temporarily unavailable.' },
      status: :service_unavailable
    )
  end
end

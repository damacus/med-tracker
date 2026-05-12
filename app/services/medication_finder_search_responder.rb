# frozen_string_literal: true

class MedicationFinderSearchResponder
  Result = Data.define(:body, :status)

  def initialize(search: NhsDmd::Search.new, medication_scope: Medication.none, stock_match_resolver: nil)
    @search = search
    @stock_match_resolver = stock_match_resolver || MedicationStockMatchResolver.new(scope: medication_scope)
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
        results: result.results.map { |search_result| result_payload(search_result, result.barcode) },
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

  def result_payload(search_result, barcode)
    search_result.to_h.tap do |payload|
      medication = existing_medication_for(search_result, barcode)
      payload[:existing_medication] = existing_medication_payload(medication) if medication
    end
  end

  def existing_medication_for(search_result, barcode)
    @stock_match_resolver.call(
      barcode: search_result.barcode.presence || barcode,
      code: search_result.code,
      system: search_result.system,
      concept_class: search_result.concept_class,
      name: search_result.name,
      display: search_result.display,
      package_unit: search_result.package_unit
    )
  end

  def existing_medication_payload(medication)
    {
      id: medication.id,
      name: medication.display_name,
      location: medication.location.name,
      path: Rails.application.routes.url_helpers.medication_path(medication)
    }
  end
end

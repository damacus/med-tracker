# frozen_string_literal: true

class MedicationFinderSearchResponder
  Result = Data.define(:body, :status)

  def initialize(search: NhsDmd::Search.new, medication_scope: Medication.none, stock_match_resolver: nil,
                 interaction_lookup: nil)
    @search = search
    @stock_match_resolver = stock_match_resolver || MedicationStockMatchResolver.new(scope: medication_scope)
    @interaction_lookup = interaction_lookup || MedicationInteractionLookup.new(medication_scope: medication_scope)
  end

  def call(query:, form: nil, strength: nil, permissions: {})
    normalized_query = query.to_s.strip
    return Result.new(body: { results: [], permissions: permissions }, status: :ok) if normalized_query.blank?

    result = @search.call(normalized_query)
    return unavailable_response unless result&.success?

    successful_response(query: normalized_query, result: result, form: form, strength: strength,
                        permissions: permissions)
  rescue StandardError => e
    Rails.logger.error("Medication finder search failed: #{e.class}: #{e.message}")
    unavailable_response
  end

  private

  def successful_response(query:, result:, form:, strength:, permissions:)
    normalized_form = NhsDmd::DosageFormFilter.normalize(form)
    results = NhsDmd::DosageFormFilter.filter(result.results, form)
    normalized_strength = NhsDmd::StrengthFilter.normalize(strength)
    results = NhsDmd::StrengthFilter.filter(results, strength)

    Result.new(
      body: {
        results: results.map { |search_result| result_payload(search_result, result.barcode) },
        query: result.resolved_query.presence || query,
        barcode: result.barcode,
        form: normalized_form,
        strength: normalized_strength,
        permissions: permissions
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
      review_result = @interaction_lookup.call(search_result)
      payload[:existing_medication] = existing_medication_payload(medication) if medication
      payload[:review_prompts] = review_result.visible_prompts
      payload[:review_prompt_filter] = { hidden_count: review_result.hidden_count }
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
      path: medication_path(medication),
      refill_path: refill_medication_path(medication),
      current_supply: MedicationStockQuantityFormatter.format(medication.current_supply)
    }
  end

  def medication_path(medication)
    route_helpers.medication_path(household_slug_for(medication), medication)
  end

  def refill_medication_path(medication)
    route_helpers.refill_medication_path(household_slug_for(medication), medication)
  end

  def household_slug_for(medication)
    Current.household&.slug || medication.household&.slug
  end

  def route_helpers
    Rails.application.routes.url_helpers
  end
end

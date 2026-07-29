# frozen_string_literal: true

class MedicationFinderSearchResponder
  Result = Data.define(:body, :status)

  def initialize(search: NhsDmd::Search.new, medication_scope: Medication.none, stock_match_resolver: nil,
                 interaction_lookup: nil, trade_family_resolver: nil)
    @search = search
    @stock_match_resolver = stock_match_resolver || MedicationStockMatchResolver.new(scope: medication_scope)
    @interaction_lookup = interaction_lookup || MedicationInteractionLookup.new(medication_scope: medication_scope)
    @trade_family_resolver = trade_family_resolver || MedicationTradeFamilyResolver.new(scope: medication_scope)
  end

  def call(query:, form: nil, strength: nil, permissions: {})
    @review_enrichment_unavailable = false
    @review_enrichment_failure_logged = false
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
    Result.new(
      body: successful_response_body(query:, result:, form:, strength:, permissions:),
      status: :ok
    )
  end

  def successful_response_body(query:, result:, form:, strength:, permissions:)
    normalized_form = NhsDmd::DosageFormFilter.normalize(form)
    normalized_strength = NhsDmd::StrengthFilter.normalize(strength)
    results = filtered_results(result.results, form:, strength:)

    {
      results: results.map { |search_result| result_payload(search_result, result.barcode) },
      review_guidance: review_guidance_payload,
      query: result.resolved_query.presence || query,
      barcode: result.barcode,
      barcode_resolution: barcode_resolution(result),
      form: normalized_form,
      strength: normalized_strength,
      permissions: permissions
    }
  end

  def unavailable_response
    Result.new(
      body: { results: [], error: 'Medication search is temporarily unavailable.' },
      status: :service_unavailable
    )
  end

  def barcode_resolution(result)
    return if result.barcode.blank?

    { status: 'resolved', source: result.barcode_source }
  end

  def filtered_results(results, form:, strength:)
    results = NhsDmd::DosageFormFilter.filter(results, form)
    NhsDmd::StrengthFilter.filter(results, strength)
  end

  def result_payload(search_result, barcode)
    search_result.to_h.tap do |payload|
      medication = existing_medication_for(search_result, barcode)
      payload[:existing_medication] = existing_medication_payload(medication) if medication
      payload[:related_medications] = related_medication_payloads(search_result, excluding: medication)
      payload.merge!(review_prompt_payload(search_result))
    end
  end

  def review_prompt_payload(search_result)
    review_result = @interaction_lookup.call(search_result)
    {
      review_prompts: review_result.visible_prompts,
      review_prompt_filter: { hidden_count: review_result.hidden_count }
    }
  rescue StandardError => e
    @review_enrichment_unavailable = true
    log_review_enrichment_failure(e)
    unavailable_review_prompt_payload
  end

  def unavailable_review_prompt_payload
    {
      review_prompts: [],
      review_prompt_filter: { hidden_count: 0 }
    }
  end

  def review_guidance_payload
    { status: @review_enrichment_unavailable ? 'unavailable' : 'available' }
  end

  def log_review_enrichment_failure(error)
    return if @review_enrichment_failure_logged

    @review_enrichment_failure_logged = true
    Rails.logger.error("Medication review enrichment failed: #{error.class}")
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

  def related_medication_payloads(search_result, excluding:)
    @trade_family_resolver.call(
      trade_family_code: trade_family_code(search_result),
      excluding: excluding
    ).map { |medication| existing_medication_payload(medication) }
  end

  def trade_family_code(search_result)
    trade_family = search_result.trade_family
    trade_family[:code] || trade_family['code'] if trade_family
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

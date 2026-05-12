# frozen_string_literal: true

module MedicationFormContext
  extend ActiveSupport::Concern

  SCHEDULE_CONFIG_KEYS = [
    :schedule_type,
    :frequency,
    :as_needed,
    :tapering_plan,
    { times: [] },
    { weekdays: [] },
    { dates: [] },
    { taper_steps: %i[
      start_date
      end_date
      amount
      unit
      frequency
      max_daily_doses
      min_hours_between_doses
    ] }
  ].freeze

  private

  def available_locations
    LocationsQuery.new(scope: policy_scope(Location)).options
  end

  def available_people
    policy_scope(Person).order(:name)
  end

  def primary_location
    PrimaryLocationQuery.new(person: current_user&.person).call
  end

  def medication_params
    params.require(:medication).permit(
      :name,
      :friendly_name,
      :barcode,
      :dmd_code,
      :dmd_system,
      :dmd_concept_class,
      :category,
      :description,
      :dosage_amount,
      :dosage_unit,
      :current_supply,
      :reorder_threshold,
      :warnings,
      :location_id,
      :default_schedule_type,
      :default_schedule_config,
      default_schedule_config: SCHEDULE_CONFIG_KEYS,
      dosage_records_attributes: %i[
        id
        amount
        unit
        frequency
        description
        default_for_adults
        default_for_children
        default_max_daily_doses
        default_min_hours_between_doses
        default_dose_cycle
        current_supply
        reorder_threshold
        _destroy
      ]
    ).tap { |permitted| normalize_default_schedule_config(permitted) }
  end

  def onboarding_schedule_params
    params.require(:onboarding_schedule).permit(
      :person_id,
      :schedule_type,
      :frequency,
      :start_date,
      :end_date,
      :max_daily_doses,
      :min_hours_between_doses,
      :dose_cycle,
      :schedule_config,
      schedule_config: SCHEDULE_CONFIG_KEYS
    )
  end

  def normalize_default_schedule_config(permitted)
    return unless permitted.key?(:default_schedule_config)

    permitted[:default_schedule_config] = normalized_schedule_config_value(permitted[:default_schedule_config])
  end

  def normalized_schedule_config_value(raw)
    return {} if raw.blank?
    return raw.permit(*SCHEDULE_CONFIG_KEYS).to_h if raw.respond_to?(:permit)
    return raw.to_h if raw.respond_to?(:to_h)

    JSON.parse(raw.to_s)
  rescue JSON::ParserError
    {}
  end

  def onboarding_builder
    @onboarding_builder ||= MedicationOnboardingBuilder.new
  end

  def medication_finder_search_responder
    @medication_finder_search_responder ||= MedicationFinderSearchResponder.new(
      medication_scope: policy_scope(Medication)
    )
  end
end

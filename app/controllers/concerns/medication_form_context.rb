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
    params.expect(
      medication: [
        :name,
        :friendly_name,
        :barcode,
        :dmd_code,
        :dmd_system,
        :dmd_concept_class,
        :category,
        :description,
        :dose_amount,
        :dose_unit,
        :current_supply,
        :reorder_threshold,
        :warnings,
        :location_id,
        :default_schedule_type,
        :default_schedule_config,
        { default_schedule_config: SCHEDULE_CONFIG_KEYS },
        { dosage_records_attributes: [%i[
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
        ]] }
      ]
    ).tap do |permitted|
      MedicationParamsNormalizer.call(permitted, schedule_config_keys: SCHEDULE_CONFIG_KEYS)
      constrain_medication_location!(permitted)
    end
  end

  def constrain_medication_location!(permitted)
    location_id = permitted[:location_id].presence
    return if location_id.blank?

    permitted[:location_id] = policy_scope(Location).find(location_id).id
  end

  def onboarding_schedule_params
    params.expect(
      onboarding_schedule: [
        :person_id,
        :schedule_type,
        :frequency,
        :start_date,
        :end_date,
        :max_daily_doses,
        :min_hours_between_doses,
        :dose_cycle,
        :schedule_config,
        { schedule_config: SCHEDULE_CONFIG_KEYS }
      ]
    )
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

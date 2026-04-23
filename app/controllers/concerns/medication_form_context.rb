# frozen_string_literal: true

module MedicationFormContext
  extend ActiveSupport::Concern

  private

  def available_locations
    LocationsQuery.new(scope: policy_scope(Location)).options
  end

  def primary_location
    PrimaryLocationQuery.new(person: current_user&.person).call
  end

  def medication_params
    params.require(:medication).permit(
      :name,
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
    )
  end

  def onboarding_builder
    @onboarding_builder ||= MedicationOnboardingBuilder.new
  end

  def medication_finder_search_responder
    @medication_finder_search_responder ||= MedicationFinderSearchResponder.new
  end
end

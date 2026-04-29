# frozen_string_literal: true

module MedicationAiSuggestionConfirmation
  extend ActiveSupport::Concern

  private

  def reject_unconfirmed_ai_medication_suggestion?
    return false unless unconfirmed_ai_medication_suggestion?

    @medication.errors.add(:base, ai_medication_suggestion_confirmation_message)
    render_create_failure
    true
  end

  def unconfirmed_ai_medication_suggestion?
    params[:ai_medication_suggestion_applied].present? && params[:ai_medication_suggestion_confirmed] != '1'
  end

  def ai_medication_suggestion_confirmation_message
    'Check AI suggestions against the packet, leaflet, or linked guidance before saving.'
  end

  def render_create_failure
    if params[:wizard] == 'true'
      render wizard_wrapper_class.new(
        medication: @medication,
        locations: available_locations,
        people: available_people,
        current_user: current_user
      ), status: :unprocessable_content
    else
      render Components::Medications::FormView.new(
        medication: @medication,
        locations: available_locations,
        title: t('medications.form.new_title'),
        subtitle: t('medications.form.new_subtitle')
      ), status: :unprocessable_content
    end
  end
end

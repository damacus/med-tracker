# frozen_string_literal: true

module PersonMedicationFormRenderable
  extend ActiveSupport::Concern

  private

  def prepare_new_person_medication
    @person_medication = @person.person_medications.build
    if medication_options_query.include?(params[:medication_id])
      @person_medication.medication_id = params[:medication_id]
    end
    @medications = medication_options_query.call
  end

  def render_person_medication_form(title:, editing: false, back_path: nil, status: :ok)
    is_modal = request.headers['Turbo-Frame'] == 'modal'

    respond_to do |format|
      format.html do
        if is_modal
          render person_medication_modal(title: title, editing: editing, back_path: back_path), layout: false, status: status
        else
          render person_medication_form_view(editing: editing), status: status
        end
      end
      format.turbo_stream do
        render turbo_stream: turbo_stream.replace(
          'modal',
          person_medication_modal(title: title, editing: editing, back_path: back_path)
        ), status: status
      end
    end
  end

  def person_medication_modal(title:, editing: false, back_path: nil)
    Components::PersonMedications::Modal.new(
      person_medication: @person_medication,
      person: @person,
      medications: @medications,
      title: title,
      editing: editing,
      back_path: back_path
    )
  end

  def person_medication_form_view(editing: false)
    Components::PersonMedications::FormView.new(
      person_medication: @person_medication,
      person: @person,
      medications: @medications,
      editing: editing
    )
  end

  def save_person_medication?
    return true if explicit_dose_submitted? && @person_medication.save

    add_explicit_dose_errors unless explicit_dose_submitted?
    false
  end

  def render_person_medication_create_failure
    render_person_medication_form(
      title: t('person_medications.modal.new_title', person: @person.name),
      status: :unprocessable_content
    )
  end

  def render_person_medication_update_failure
    render_person_medication_form(
      title: t('person_medications.modal.edit_title', person: @person.name),
      editing: true,
      status: :unprocessable_content
    )
  end
end

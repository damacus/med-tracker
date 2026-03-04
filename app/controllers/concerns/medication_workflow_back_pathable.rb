# frozen_string_literal: true

module MedicationWorkflowBackPathable
  extend ActiveSupport::Concern

  private

  def modal_back_path(person)
    return unless request.headers['Turbo-Frame'] == 'modal'

    add_medication_person_path(person, source: :workflow)
  end
end

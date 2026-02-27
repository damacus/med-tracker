# frozen_string_literal: true

module Components
  module PersonMedications
    # Modal component for person medication form
    class Modal < Components::Base
      include Phlex::Rails::Helpers::FormWith
      include Phlex::Rails::Helpers::TurboFrameTag

      attr_reader :person_medication, :person, :medications, :title, :editing

      def initialize(person_medication:, person:, medications:, title: nil, editing: false)
        @person_medication = person_medication
        @person = person
        @medications = medications
        @editing = editing
        @title = title || (editing ? "Edit Medication for #{person.name}" : "Add Medication for #{person.name}")
        super()
      end

      def view_template
        turbo_frame_tag 'modal' do
          render ::Components::Modal.new(title: title,
                                         subtitle: 'Add a vitamin, supplement, or over-the-counter medication') do
            render_form
          end
        end
      end

      private

      def render_form
        form_with(
          model: person_medication,
          url: form_url,
          method: editing ? :patch : :post,
          class: 'space-y-6'
        ) do
          render_form_fields
          render_actions
        end
      end

      def form_url
        if editing
          person_person_medication_path(person, person_medication)
        else
          person_person_medications_path(person)
        end
      end

      def render_form_fields
        render FormFields.new(person_medication: person_medication, medications: medications, editing: editing)
      end

      def render_actions
        div(class: 'flex justify-end gap-3 pt-4') do
          Button(variant: :ghost, data: { action: 'click->modal#close' }) { 'Cancel' }
          Button(type: :submit, variant: :primary) do
            editing ? 'Save Changes' : 'Add Medication'
          end
        end
      end
    end
  end
end

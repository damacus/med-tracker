# frozen_string_literal: true

module Components
  module PersonMedicines
    # Modal component for person medicine form
    class Modal < Components::Base
      include Phlex::Rails::Helpers::FormWith
      include Phlex::Rails::Helpers::TurboFrameTag

      attr_reader :person_medicine, :person, :medicines, :title, :editing

      def initialize(person_medicine:, person:, medicines:, title: nil, editing: false)
        @person_medicine = person_medicine
        @person = person
        @medicines = medicines
        @editing = editing
        @title = title || (editing ? "Edit Medicine for #{person.name}" : "Add Medicine for #{person.name}")
        super()
      end

      def view_template
        turbo_frame_tag 'person_medicine_modal' do
          Dialog(open: true) do
            DialogContent(size: :lg) do
              DialogHeader do
                DialogTitle { title }
                DialogDescription { 'Add a vitamin, supplement, or over-the-counter medicine' }
              end
              DialogMiddle do
                render_form
              end
            end
          end
        end
      end

      private

      def render_form
        form_with(
          model: person_medicine,
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
          person_person_medicine_path(person, person_medicine)
        else
          person_person_medicines_path(person)
        end
      end

      def render_form_fields
        render FormFields.new(person_medicine: person_medicine, medicines: medicines, editing: editing)
      end

      def render_actions
        div(class: 'flex justify-end gap-3 pt-4') do
          Link(
            href: person_path(person),
            variant: :outline,
            data: { turbo_frame: 'person_medicine_modal' }
          ) { 'Cancel' }
          Button(type: :submit, variant: :primary) do
            editing ? 'Save Changes' : 'Add Medicine'
          end
        end
      end
    end
  end
end

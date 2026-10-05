# frozen_string_literal: true

module Components
  module MedicationAssignments
    class Modal < Components::Base
      include Phlex::Rails::Helpers::TurboFrameTag
      include RubyUI

      attr_reader :assignment, :person, :medications, :back_path, :return_to

      def initialize(assignment:, person:, medications:, back_path: nil, return_to: nil)
        @assignment = assignment
        @person = person
        @medications = medications
        @back_path = back_path
        @return_to = return_to
        super()
      end

      def view_template
        turbo_frame_tag 'modal' do
          Dialog(open: true) do
            DialogContent(size: :md) do
              DialogHeader do
                render_back_link if back_path
                DialogTitle { t('person_medications.form.add_medication_for', person: person.name) }
                DialogDescription { t('medication_assignments.modal.subtitle') }
              end
              DialogMiddle do
                render Form.new(
                  assignment: assignment,
                  person: person,
                  medications: medications,
                  modal: true,
                  navigation: { back_path: back_path, return_to: return_to }
                )
              end
            end
          end
        end
      end

      private

      def render_back_link
        m3_link(
          href: back_path,
          variant: :text,
          size: :sm,
          data: { turbo_frame: 'modal' },
          class: 'mb-2 h-auto min-h-0 justify-start p-0 text-sm text-on-surface-variant hover:text-foreground'
        ) do
          plain t('medication_workflow.back')
        end
      end
    end
  end
end

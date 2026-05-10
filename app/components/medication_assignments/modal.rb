# frozen_string_literal: true

module Components
  module MedicationAssignments
    class Modal < Components::Base
      include Phlex::Rails::Helpers::TurboFrameTag
      include RubyUI

      attr_reader :assignment, :person, :medications, :back_path

      def initialize(assignment:, person:, medications:, back_path: nil)
        @assignment = assignment
        @person = person
        @medications = medications
        @back_path = back_path
        super()
      end

      def view_template
        turbo_frame_tag("modal") do
          Dialog(open: true) do
            DialogContent(size: :md) do
              DialogHeader do
                render_back_link if back_path
                DialogTitle { t("person_medications.form.add_medication_for", person: person.name) }
                DialogDescription { t("medication_assignments.modal.subtitle") }
              end

              DialogMiddle do
                render(
                  Form.new(
                    assignment: assignment,
                    person: person,
                    medications: medications,
                    modal: true,
                    back_path: back_path
                  )
                )
              end
            end
          end
        end
      end

      private

      def render_back_link
        a(
          href: back_path,
          data: {turbo_frame: "modal"},
          class: "inline-flex items-center text-sm text-on-surface-variant hover:text-foreground " \
            "transition-colors mb-2 no-underline"
        ) do
          plain(t("medication_workflow.back"))
        end
      end
    end
  end
end

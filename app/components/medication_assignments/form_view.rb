# frozen_string_literal: true

module Components
  module MedicationAssignments
    class FormView < Components::Base
      attr_reader :assignment, :person, :medications

      def initialize(assignment:, person:, medications:)
        @assignment = assignment
        @person = person
        @medications = medications
        super()
      end

      def view_template
        div(class: "container mx-auto px-4 py-8 max-w-2xl") do
          div(class: "mb-8") do
            m3_text(
              size: "2",
              weight: "medium",
              class: "uppercase tracking-wide text-on-surface-variant mb-2"
            ) do
              t("person_medications.form.add_medication")
            end

            m3_heading(level: 1) do
              t("person_medications.form.add_medication_for", person: person.name)
            end
          end

          render(Form.new(assignment: assignment, person: person, medications: medications))
        end
      end
    end
  end
end

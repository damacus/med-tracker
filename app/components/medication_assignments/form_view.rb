# frozen_string_literal: true

module Components
  module MedicationAssignments
    class FormView < Components::Base
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
        div(class: 'container mx-auto px-4 py-8 max-w-2xl') do
          div(class: 'mb-8') do
            render_back_link if back_path
            m3_text(size: '2', weight: 'medium',
                    class: 'uppercase tracking-wide text-on-surface-variant mb-2') do
              t('person_medications.form.add_medication')
            end
            m3_heading(level: 1) do
              t('person_medications.form.add_medication_for', person: person.name)
            end
          end
          render Form.new(
            assignment: assignment,
            person: person,
            medications: medications,
            navigation: { back_path: back_path, return_to: return_to }
          )
        end
      end

      private

      def render_back_link
        m3_link(href: back_path, variant: :text, size: :sm, class: 'mb-2') do
          t('medication_workflow.back')
        end
      end
    end
  end
end

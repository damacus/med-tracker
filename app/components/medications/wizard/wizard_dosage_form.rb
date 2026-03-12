# frozen_string_literal: true

module Components
  module Medications
    module Wizard
      class WizardDosageForm < Components::Dosages::Form
        def view_template
          form_with(
            model: [medication, dosage],
            class: 'space-y-5'
          ) do |_f|
            input(type: 'hidden', name: 'wizard', value: 'true')
            render_errors if dosage.errors.any?
            render_basic_fields
            render_divider
            render_scheduling_defaults
            render_default_flags
            render_actions
          end
        end

        private

        def render_actions
          div(class: 'flex gap-3 justify-end pt-2') do
            Button(type: :submit, variant: :primary) do
              dosage.new_record? ? 'Add Dosage' : 'Update Dosage'
            end
          end
        end
      end
    end
  end
end

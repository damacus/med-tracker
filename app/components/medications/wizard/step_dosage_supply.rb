# frozen_string_literal: true

module Components
  module Medications
    module Wizard
      class StepDosageSupply < Components::Base
        include FieldHelpers

        attr_reader :medication, :locations

        def initialize(medication:, locations: [])
          @medication = medication
          @locations = locations
          super()
        end

        def view_template
          div(class: 'space-y-6') do
            div(class: 'space-y-1 mb-2') do
              Heading(level: 3, size: '5', class: 'font-bold tracking-tight text-foreground') do
                t('forms.medications.dosage_and_supply')
              end
              Text(size: '2', class: 'text-muted-foreground') do
                'Set the default dosage and track your supply'
              end
            end

            div(class: 'grid grid-cols-1 sm:grid-cols-2 gap-6') do
              render_dosage_amount_field
              render_dosage_unit_field
            end

            div(class: 'h-px bg-border w-full')

            div(class: 'grid grid-cols-1 sm:grid-cols-2 gap-6') do
              render_current_supply_field
              render_reorder_threshold_field
            end
          end
        end
      end
    end
  end
end

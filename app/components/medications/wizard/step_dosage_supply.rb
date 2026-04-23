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
              m3_heading(level: 3, size: '5', class: 'font-bold tracking-tight text-foreground') do
                t('forms.medications.dosage_and_supply')
              end
              m3_text(size: '2', class: 'text-on-surface-variant') do
                'Set a primary dose option, then confirm the starting supply.'
              end
            end

            div(class: 'space-y-4') do
              m3_heading(level: 4, size: '4', class: 'font-bold tracking-tight text-foreground') { 'How to take it' }
              m3_text(size: '2', class: 'text-on-surface-variant') do
                'This becomes the primary dose option saved with the medication.'
              end
              render_primary_dosage_option_fields
            end

            div(class: 'h-px bg-border w-full')

            div(class: 'space-y-4') do
              m3_heading(level: 4, size: '4', class: 'font-bold tracking-tight text-foreground') { 'Supply setup' }
              m3_text(size: '2', class: 'text-on-surface-variant') do
                'Track how much you have in the pack and when to reorder.'
              end
              div(class: 'grid grid-cols-1 sm:grid-cols-2 gap-6') do
                render_current_supply_field
                render_reorder_threshold_field
              end
            end

            render_suggested_dosage_records_section
          end
        end
      end
    end
  end
end

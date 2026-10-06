# frozen_string_literal: true

module Components
  module Medications
    module Wizard
      class StepSupply < Components::Base
        include FieldHelpers

        attr_reader :medication

        def initialize(medication:)
          @medication = medication
          super()
        end

        def view_template
          div(class: 'space-y-6') do
            div(class: 'space-y-1 mb-2') do
              m3_heading(level: 3, size: '5', class: 'font-bold tracking-tight text-foreground') do
                t('forms.medications.wizard.supply.title')
              end
              m3_text(size: '2', class: 'text-on-surface-variant') do
                t('forms.medications.wizard.supply.description')
              end
            end

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

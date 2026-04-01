# frozen_string_literal: true

module Components
  module Medications
    module Wizard
      class StepWarnings < Components::Base
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
              div(class: 'flex items-center gap-2') do
                render Icons::AlertCircle.new(size: 20, class: 'text-on-error-container')
                Heading(level: 3, size: '5', class: 'font-bold tracking-tight text-foreground') do
                  t('forms.medications.warnings')
                end
              end
              Text(size: '2', class: 'text-muted-foreground') do
                'Add any safety warnings or important notes'
              end
            end

            render_warnings_field

            div(class: 'rounded-2xl bg-warning-container border border-warning p-4') do
              div(class: 'flex gap-3') do
                render Icons::AlertCircle.new(size: 16, class: 'text-on-warning-container mt-0.5 shrink-0')
                Text(size: '2', class: 'text-on-warning-container') do
                  'Warnings will be displayed prominently on the medication profile ' \
                    'and when administering doses.'
                end
              end
            end
          end
        end
      end
    end
  end
end

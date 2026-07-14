# frozen_string_literal: true

module Components
  module Medications
    module Wizard
      class StepBasicInfo < Components::Base
        include FieldHelpers

        attr_reader :medication, :locations, :ai_medication_help_enabled

        def initialize(medication:, locations:, ai_medication_help_enabled: false)
          @medication = medication
          @locations = locations
          @ai_medication_help_enabled = ai_medication_help_enabled
          super()
        end

        def view_template
          div(class: 'space-y-6') do
            div(class: 'space-y-1 mb-2') do
              m3_heading(level: 2, size: '5', class: 'font-bold tracking-tight text-foreground') do
                t('forms.medications.wizard.details.title')
              end
              m3_text(size: '2', class: 'text-on-surface-variant') do
                t('forms.medications.wizard.details.description')
              end
            end

            render_location_field
            render_ai_medication_help if ai_medication_help_enabled
            render_name_field
            render_category_field
            render_description_field
          end
        end

        private

        def render_ai_medication_help
          div(
            class: 'rounded-3xl border border-primary/20 bg-primary/5 p-4 space-y-3',
            data: { 'ai-medication-help-target': 'panel' }
          ) do
            div(class: 'flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between') do
              div(class: 'space-y-1') do
                m3_heading(level: 4, size: '4', class: 'font-bold text-foreground') { 'Help fill from trusted sources' }
                m3_text(size: '2', class: 'text-on-surface-variant') do
                  'We can look for packet or leaflet guidance and draft fields for you to check.'
                end
              end
              m3_button(
                type: :button,
                variant: :outlined,
                class: 'w-full shrink-0 sm:w-auto',
                data: {
                  action: 'click->ai-medication-help#suggest',
                  'ai-medication-help-target': 'button'
                }
              ) { 'Help fill this in' }
            end
            div(
              class: 'hidden rounded-2xl border border-outline-variant/60 bg-surface-container-low p-3 text-sm',
              data: { 'ai-medication-help-target': 'status' }
            )
          end
        end
      end
    end
  end
end

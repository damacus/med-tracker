# frozen_string_literal: true

module Components
  module Medications
    module Wizard
      class FullPageWrapper < Components::Base
        attr_reader :medication, :locations, :variant

        def initialize(medication:, locations:)
          @medication = medication
          @locations = locations
          @variant = 'fullpage'
          super()
        end

        def view_template
          div(class: 'container mx-auto px-4 py-12 max-w-2xl') do
            render_header

            m3_card(variant: :elevated, class: 'overflow-visible border-none shadow-elevation-3 rounded-[2.5rem]') do
              div(class: 'p-10') do
                render StepContent.new(
                  medication: medication,
                  locations: locations,
                  variant: variant
                )
              end
            end
          end
        end

        private

        def render_header
          div(class: 'text-center mb-10 space-y-2') do
            div(
              class: 'mx-auto w-16 h-16 rounded-[1.5rem] bg-primary/10 flex items-center justify-center ' \
                     'text-primary shadow-inner mb-6'
            ) do
              render Icons::Pill.new(size: 32)
            end
            m3_text(variant: :label_medium, class: 'uppercase tracking-[0.2em] font-black opacity-40') do
              t('forms.medications.inventory_management')
            end
            m3_heading(variant: :display_small, level: 1, class: 'font-black tracking-tight text-foreground') do
              t('medications.form.new_title')
            end
            m3_text(variant: :body_large, class: 'text-on-surface-variant') do
              t('medications.form.new_subtitle')
            end
          end
        end
      end
    end
  end
end

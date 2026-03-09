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

            Card(class: 'overflow-visible border-none shadow-2xl rounded-[2.5rem] bg-white') do
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
            Text(size: '2', weight: 'black', class: 'uppercase tracking-[0.2em] font-bold opacity-40') do
              t('forms.medications.inventory_management')
            end
            Heading(level: 1, size: '8', class: 'font-black tracking-tight text-slate-900') do
              t('medications.form.new_title')
            end
            Text(size: '3', class: 'text-slate-400') do
              t('medications.form.new_subtitle')
            end
          end
        end
      end
    end
  end
end

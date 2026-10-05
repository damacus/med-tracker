# frozen_string_literal: true

module Components
  module Medications
    module Wizard
      class FullPageWrapper < Components::Base
        attr_reader :medication, :locations, :people, :current_user, :variant

        def initialize(medication:, locations:, people:, current_user: nil)
          @medication = medication
          @locations = locations
          @people = people
          @current_user = current_user
          @variant = 'fullpage'
          super()
        end

        def view_template
          div(class: 'container mx-auto max-w-2xl px-3 py-6 sm:px-4 sm:py-12') do
            render_header

            m3_card(variant: :elevated, class: 'overflow-visible border-none shadow-elevation-3 rounded-shape-xl') do
              div(class: 'p-5 sm:p-10') do
                render StepContent.new(
                  medication: medication,
                  locations: locations,
                  people: people,
                  current_user: current_user,
                  variant: variant
                )
              end
            end
          end
        end

        private

        def render_header
          div(class: 'mb-6 space-y-2 text-center sm:mb-10') do
            div(
              class: 'mx-auto w-16 h-16 rounded-shape-xl bg-primary/10 flex items-center justify-center ' \
                     'text-primary shadow-inner mb-4 sm:mb-6'
            ) do
              render Components::Shared::MedicationIcon.new(medication: medication, size: 32)
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

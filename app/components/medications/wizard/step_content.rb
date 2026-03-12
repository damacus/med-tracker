# frozen_string_literal: true

module Components
  module Medications
    module Wizard
      class StepContent < Components::Base
        include Phlex::Rails::Helpers::FormWith

        attr_reader :medication, :locations, :variant

        def initialize(medication:, locations:, variant: 'fullpage')
          @medication = medication
          @locations = locations
          @variant = variant
          super()
        end

        def view_template
          div(
            id: 'wizard-content',
            data: {
              controller: 'wizard',
              wizard_current_value: 0
            }
          ) do
            render StepIndicator.new

            form_with(
              model: medication,
              class: 'space-y-8',
              turbo_frame: frame_target,
              data: { testid: 'medication-wizard-form' }
            ) do |_form|
              render_errors if medication.errors.any?
              input(type: 'hidden', name: 'wizard', value: 'true')

              render_step_panels
              render_navigation
            end
          end
        end

        private

        def frame_target
          overlay_variant? ? 'modal' : nil
        end

        def overlay_variant?
          variant.in?(%w[modal slideover])
        end

        def render_errors
          render RubyUI::Alert.new(variant: :destructive, class: 'mb-4 rounded-2xl border-none shadow-sm') do
            div(class: 'flex items-start gap-3') do
              render Icons::AlertCircle.new(size: 20)
              div do
                Heading(level: 2, size: '3', class: 'font-bold mb-1') do
                  t('forms.medications.validation_errors', count: medication.errors.count)
                end
                ul(class: 'text-sm opacity-90 list-disc pl-4 space-y-1') do
                  medication.errors.full_messages.each do |message|
                    li { message }
                  end
                end
              end
            end
          end
        end

        def render_step_panels
          div(data: { wizard_target: 'step' }) do
            render StepBasicInfo.new(medication: medication, locations: locations)
          end

          div(class: 'hidden', data: { wizard_target: 'step' }) do
            render StepDosageSupply.new(medication: medication)
          end

          div(class: 'hidden', data: { wizard_target: 'step' }) do
            render StepWarnings.new(medication: medication)
          end
        end

        def render_navigation
          div(class: 'flex items-center justify-between pt-6 border-t border-slate-100') do
            div(class: 'flex items-center gap-3') do
              Button(
                type: :button,
                variant: :ghost,
                class: 'font-bold text-slate-400 hover:text-slate-600 invisible',
                data: {
                  wizard_target: 'prevButton',
                  action: 'click->wizard#prev'
                }
              ) do
                render Icons::ChevronLeft.new(size: 16, class: 'mr-1')
                plain 'Back'
              end

              if overlay_variant?
                a(
                  href: medications_path,
                  class: 'text-sm font-bold text-slate-400 hover:text-slate-600',
                  data: { turbo_frame: '_top' }
                ) { 'Cancel' }
              end
            end

            div(class: 'flex gap-3') do
              Button(
                type: :button,
                variant: :primary,
                size: :lg,
                class: 'px-8 rounded-2xl shadow-lg shadow-primary/20',
                data: {
                  wizard_target: 'nextButton',
                  action: 'click->wizard#next'
                }
              ) do
                plain 'Continue'
              end

              Button(
                type: :submit,
                variant: :primary,
                size: :lg,
                class: 'px-8 rounded-2xl shadow-lg shadow-primary/20 hidden',
                data: { wizard_target: 'submitButton' }
              ) do
                plain t('forms.medications.save_medication')
              end
            end
          end
        end
      end
    end
  end
end

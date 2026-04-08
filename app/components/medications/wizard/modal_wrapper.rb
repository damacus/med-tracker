# frozen_string_literal: true

module Components
  module Medications
    module Wizard
      class ModalWrapper < Components::Base
        include Phlex::Rails::Helpers::TurboFrameTag

        attr_reader :medication, :locations, :variant

        def initialize(medication:, locations:)
          @medication = medication
          @locations = locations
          @variant = 'modal'
          super()
        end

        def view_template
          turbo_frame_tag 'modal' do
            a(
              href: medications_path,
              class: 'fixed inset-0 z-50 bg-black/60 backdrop-blur-sm',
              data: { turbo_frame: '_top' },
              aria_label: 'Close'
            )

            div(
              class: 'fixed left-1/2 top-1/2 z-50 w-full max-w-2xl -translate-x-1/2 -translate-y-1/2 ' \
                     'rounded-[2.5rem] bg-surface-container-lowest shadow-2xl ' \
                     'overflow-hidden max-h-[90vh] overflow-y-auto'
            ) do
              a(
                href: medications_path,
                data: { turbo_frame: '_top' },
                class: 'absolute top-4 right-4 w-10 h-10 rounded-full bg-surface-container hover:bg-accent ' \
                       'flex items-center justify-center text-muted-foreground transition-colors z-10',
                aria_label: 'Close'
              ) do
                render Icons::X.new(size: 18)
                span(class: 'sr-only') { 'Close' }
              end

              div(class: 'p-8') do
                render_header
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
          div(class: 'mb-8 space-y-2') do
            div(
              class: 'w-12 h-12 rounded-2xl bg-primary/10 flex items-center justify-center ' \
                     'text-primary mb-4'
            ) do
              render Icons::Pill.new(size: 24)
            end
            Heading(level: 1, size: '6', class: 'font-black tracking-tight text-foreground') do
              t('medications.form.new_title')
            end
            Text(size: '2', class: 'text-muted-foreground') do
              t('medications.form.new_subtitle')
            end
          end
        end
      end
    end
  end
end

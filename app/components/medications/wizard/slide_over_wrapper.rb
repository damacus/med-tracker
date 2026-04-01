# frozen_string_literal: true

module Components
  module Medications
    module Wizard
      class SlideOverWrapper < Components::Base
        include Phlex::Rails::Helpers::TurboFrameTag

        attr_reader :medication, :locations, :variant

        def initialize(medication:, locations:)
          @medication = medication
          @locations = locations
          @variant = 'slideover'
          super()
        end

        def view_template
          turbo_frame_tag 'modal' do
            a(
              href: medications_path,
              class: 'fixed inset-0 z-50 bg-black/40 backdrop-blur-sm',
              data: { turbo_frame: '_top' },
              aria_label: 'Close'
            )

            # Slide-over panel
            div(
              class: 'fixed inset-y-0 right-0 z-50 w-full max-w-xl bg-surface-container-lowest shadow-2xl ' \
                     'overflow-y-auto animate-slide-in-right'
            ) do
              render_close_button
              div(class: 'p-8 pt-16') do
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

        def render_close_button
          a(
            href: medications_path,
            class: 'absolute top-4 right-4 w-10 h-10 rounded-full bg-surface-container hover:bg-accent ' \
                   'flex items-center justify-center text-muted-foreground transition-colors z-10',
            data: { turbo_frame: '_top' }
          ) do
            render Icons::X.new(size: 18)
            span(class: 'sr-only') { 'Close' }
          end
        end

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

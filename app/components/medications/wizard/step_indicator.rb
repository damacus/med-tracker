# frozen_string_literal: true

module Components
  module Medications
    module Wizard
      class StepIndicator < Components::Base
        STEPS = [
          { label: 'Details', icon: '1' },
          { label: 'Dosage', icon: '2' },
          { label: 'Warnings', icon: '3' }
        ].freeze

        def view_template
          nav(class: 'mb-10', aria_label: 'Wizard progress') do
            ol(class: 'flex items-center justify-between max-w-md mx-auto') do
              STEPS.each_with_index do |step, index|
                render_step_indicator(step, index)
              end
            end
          end
        end

        private

        def render_step_indicator(step, index)
          li(
            class: 'flex-1 flex flex-col items-center relative',
            data: { wizard_target: 'indicator' }
          ) do
            render_connecting_line if index.positive?
            render_circle(step, index)
            render_label(step, index)
          end
        end

        def render_connecting_line
          div(
            class: 'absolute top-4 -left-1/2 w-full h-0.5 bg-surface-container-high ' \
                   '-z-10 transition-colors duration-300',
            data: { indicator_line: true }
          )
        end

        def render_circle(step, index)
          base = 'w-8 h-8 rounded-full flex items-center justify-center text-xs font-bold transition-all duration-300'
          active = if index.zero?
                     'bg-primary text-white ring-4 ring-primary/20 scale-110'
                   else
                     'bg-surface-container text-muted-foreground'
                   end

          div(class: "#{base} #{active}", data: { indicator_circle: true }) { step[:icon] }
        end

        def render_label(step, index)
          base = 'mt-2 text-[10px] font-bold uppercase tracking-widest transition-colors duration-300'
          color = index.zero? ? 'text-primary' : 'text-muted-foreground'

          span(class: "#{base} #{color}", data: { indicator_label: true }) { step[:label] }
        end
      end
    end
  end
end

# frozen_string_literal: true

module Components
  module Medications
    class WarningsComponent < Components::Base
      attr_reader :medication

      def initialize(medication:)
        @medication = medication
        super()
      end

      def view_template
        div(class: 'space-y-4') do
          div(class: 'flex items-center gap-2') do
            render Icons::AlertCircle.new(size: 20, class: 'text-on-error-container')
            Heading(level: 2, size: '5', class: 'font-bold tracking-tight text-on-error-container') do
              t('medications.show.safety_warnings')
            end
          end
          Card(class: 'bg-error-container border-error/20 p-8') do
            Text(size: '3', class: 'text-on-error-container leading-relaxed font-medium') { medication.warnings }
          end
        end
      end
    end
  end
end

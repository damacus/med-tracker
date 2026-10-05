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
            m3_heading(variant: :title_large, level: 2, class: 'font-bold tracking-tight text-on-error-container') do
              t('medications.show.safety_warnings')
            end
          end
          m3_card(variant: :filled, class: 'bg-error-container border-error/20 p-8') do
            m3_text(variant: :body_large, class: 'text-on-error-container leading-relaxed font-medium') do
              medication.warnings
            end
          end
        end
      end
    end
  end
end

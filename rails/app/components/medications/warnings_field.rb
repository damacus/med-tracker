# frozen_string_literal: true

module Components
  module Medications
    class WarningsField < Components::Base
      attr_reader :medication

      def initialize(medication:)
        @medication = medication
        super()
      end

      def view_template
        div(class: 'space-y-2') do
          render RubyUI::FormFieldLabel.new(
            for: 'medication_warnings',
            class: 'text-[10px] font-black uppercase tracking-widest text-error ml-1'
          ) { t('forms.medications.warnings') }
          render RubyUI::Textarea.new(
            name: 'medication[warnings]',
            id: 'medication_warnings',
            rows: 3,
            placeholder: t('forms.medications.warnings_placeholder'),
            class: 'rounded-shape-sm border-error/20 bg-error-container/10 p-4 text-on-error-container focus:ring-2 ' \
                   'focus:ring-error/10 focus:border-error transition-all resize-none ' \
                   'placeholder:text-on-error-container/50 font-medium'
          ) { medication.warnings }
        end
      end
    end
  end
end

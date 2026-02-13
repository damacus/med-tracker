# frozen_string_literal: true

module Components
  module Prescriptions
    class FormPageBase < Components::Base
      attr_reader :prescription, :person, :medicines

      def initialize(prescription:, person:, medicines:)
        @prescription = prescription
        @person = person
        @medicines = medicines
        super()
      end

      def view_template
        div(class: 'container mx-auto px-4 py-8 max-w-4xl') do
          render_header
          render Form.new(prescription: prescription, person: person, medicines: medicines)
        end
      end

      private

      def header_label
        raise NotImplementedError
      end

      def header_title
        raise NotImplementedError
      end

      def render_header
        div(class: 'mb-8') do
          Text(size: '2', weight: 'medium', class: 'uppercase tracking-wide text-slate-500 mb-2') do
            header_label
          end
          Heading(level: 1) { header_title }
        end
      end
    end
  end
end

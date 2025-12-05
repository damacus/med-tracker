# frozen_string_literal: true

module Components
  module Prescriptions
    # Edit prescription view component
    class EditView < Components::Base
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

      def render_header
        div(class: 'mb-8') do
          p(class: 'text-sm font-medium uppercase tracking-wide text-slate-500 mb-2') do
            'Edit Prescription'
          end
          h1(class: 'text-4xl font-bold text-slate-900') do
            "Update prescription for #{person.name}"
          end
        end
      end
    end
  end
end

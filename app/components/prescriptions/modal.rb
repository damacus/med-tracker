# frozen_string_literal: true

module Components
  module Prescriptions
    # Modal component for prescription form using RubyUI Dialog
    class Modal < Components::Base
      attr_reader :prescription, :person, :medicines, :title

      def initialize(prescription:, person:, medicines:, title: nil)
        @prescription = prescription
        @person = person
        @medicines = medicines
        @title = title || "New Prescription for #{person.name}"
        super()
      end

      def view_template
        Dialog(open: true) do
          DialogContent(size: :xl) do
            DialogHeader do
              DialogTitle { title }
              DialogDescription { 'Add medication details and schedule' }
            end
            DialogMiddle do
              render Form.new(prescription: prescription, person: person, medicines: medicines)
            end
          end
        end
      end
    end
  end
end

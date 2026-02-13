# frozen_string_literal: true

module Components
  module Prescriptions
    # Edit prescription view component
    class EditView < Components::Prescriptions::FormPageBase
      private

      def header_label = 'Edit Prescription'
      def header_title = "Update prescription for #{person.name}"
    end
  end
end

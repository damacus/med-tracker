# frozen_string_literal: true

module Components
  module Prescriptions
    # New prescription view component
    class NewView < Components::Prescriptions::FormPageBase
      private

      def header_label = 'New Prescription'
      def header_title = "Add prescription for #{person.name}"
    end
  end
end

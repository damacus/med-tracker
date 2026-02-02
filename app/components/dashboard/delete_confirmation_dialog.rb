# frozen_string_literal: true

module Components
  module Dashboard
    # Renders a delete confirmation dialog for prescriptions
    class DeleteConfirmationDialog < Components::Base
      include Phlex::Rails::Helpers::FormWith

      attr_reader :prescription, :url_helpers, :button_class

      def initialize(prescription:, url_helpers:, button_class: nil)
        @prescription = prescription
        @url_helpers = url_helpers
        @button_class = button_class || default_button_class
        super()
      end

      def view_template
        AlertDialog do
          AlertDialogTrigger do
            Button(
              variant: :destructive,
              size: :sm,
              class: button_class,
              data: { test_id: "delete-prescription-#{prescription.id}" }
            ) { 'Delete' }
          end

          AlertDialogContent do
            AlertDialogHeader do
              AlertDialogTitle { 'Delete Prescription?' }
              AlertDialogDescription do
                plain "Are you sure you want to delete #{prescription.medicine.name} "
                plain "for #{prescription.person.name}? This action cannot be undone."
              end
            end

            AlertDialogFooter do
              AlertDialogCancel { 'Cancel' }
              render_delete_form
            end
          end
        end
      end

      private

      def render_delete_form
        form_with(
          url: url_helpers.person_prescription_path(prescription.person, prescription),
          method: :delete,
          class: 'inline',
          data: { turbo_frame: '_top' }
        ) do
          Button(
            variant: :destructive,
            type: :submit,
            data: { test_id: "confirm-delete-#{prescription.id}" }
          ) { 'Delete' }
        end
      end

      def default_button_class
        'inline-flex items-center justify-center rounded-full text-sm font-medium transition-colors ' \
          'min-h-[44px] min-w-[44px] px-4 py-2 bg-red-100 text-red-700 hover:bg-red-200'
      end
    end
  end
end

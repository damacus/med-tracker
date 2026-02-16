# frozen_string_literal: true

module Components
  module Dashboard
    # Renders a delete confirmation dialog for prescriptions
    class DeleteConfirmationDialog < Components::Base
      include Phlex::Rails::Helpers::FormWith
      include Phlex::Rails::Helpers::T

      attr_reader :prescription, :url_helpers, :button_class

      def initialize(prescription:, url_helpers:, button_class: nil)
        @prescription = prescription
        @url_helpers = url_helpers
        @button_class = button_class
        super()
      end

      def view_template
        AlertDialog do
          AlertDialogTrigger do
            Button(
              variant: :destructive_outline,
              size: :sm,
              class: button_class,
              data: { test_id: "delete-prescription-#{prescription.id}" }
            ) { t('dashboard.delete_confirmation.delete') }
          end

          AlertDialogContent do
            AlertDialogHeader do
              AlertDialogTitle { t('dashboard.delete_confirmation.delete_prescription') }
              AlertDialogDescription do
                plain t('dashboard.delete_confirmation.are_you_sure', medicine: prescription.medicine.name,
                                                                      person: prescription.person.name)
              end
            end

            AlertDialogFooter do
              AlertDialogCancel { t('dashboard.delete_confirmation.cancel') }
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
          ) { t('dashboard.delete_confirmation.delete') }
        end
      end
    end
  end
end

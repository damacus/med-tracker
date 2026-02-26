# frozen_string_literal: true

module Components
  module Dashboard
    # Renders a delete confirmation dialog for schedules
    class DeleteConfirmationDialog < Components::Base
      include Phlex::Rails::Helpers::FormWith

      attr_reader :schedule, :button_class

      def initialize(schedule:, button_class: nil)
        @schedule = schedule
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
              data: { test_id: "delete-schedule-#{schedule.id}" }
            ) { t('dashboard.delete_confirmation.delete') }
          end

          AlertDialogContent do
            AlertDialogHeader do
              AlertDialogTitle { t('dashboard.delete_confirmation.delete_schedule') }
              AlertDialogDescription do
                plain t('dashboard.delete_confirmation.are_you_sure', medication: schedule.medication.name,
                                                                      person: schedule.person.name)
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
          url: person_schedule_path(schedule.person, schedule),
          method: :delete,
          class: 'inline',
          data: { turbo_frame: '_top' }
        ) do
          Button(
            variant: :destructive,
            type: :submit,
            data: { test_id: "confirm-delete-#{schedule.id}" }
          ) { t('dashboard.delete_confirmation.delete') }
        end
      end
    end
  end
end

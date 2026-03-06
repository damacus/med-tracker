# frozen_string_literal: true

module Components
  module Dashboard
    # Renders a schedule row for desktop table view
    class ScheduleRow < Components::Base
      include Phlex::Rails::Helpers::FormWith
      include ScheduleHelpers

      attr_reader :person, :schedule, :current_user

      def initialize(person:, schedule:, current_user: nil)
        @person = person
        @schedule = schedule
        @current_user = current_user
        super()
      end

      def view_template
        TableRow(id: "schedule_#{schedule.id}") do
          render_person_cell
          render_medication_cell
          TableCell { format_dosage }
          TableCell { format_quantity }
          TableCell { schedule.frequency || '—' }
          TableCell { format_end_date }
          TableCell(class: 'text-center') { render_actions }
        end
      end

      private

      def render_person_cell
        TableCell(class: 'font-medium') do
          div(class: 'flex items-center gap-2') do
            render_person_avatar
            span(class: 'font-semibold text-slate-900') { person.name }
          end
        end
      end

      def render_person_avatar
        Avatar(size: :sm) do
          AvatarFallback { '👤' }
        end
      end

      def render_medication_cell
        TableCell do
          div(class: 'flex justify-between items-center w-full gap-2') do
            div(class: 'flex items-center gap-2') do
              render_medication_icon
              span(class: 'font-medium') { schedule.medication.name }
            end
            render Components::Shared::StockBadge.new(medication: schedule.medication)
          end
        end
      end

      def render_medication_icon
        div(class: 'w-8 h-8 rounded-lg flex items-center justify-center bg-success-light text-success flex-shrink-0') do
          render Icons::Pill.new(size: 16)
        end
      end

      def render_actions
        div(class: 'flex items-center justify-center gap-2') do
          render_take_now_button
          render_delete_button if can_delete?
        end
      end

      def render_take_now_button
        label = blocked_reason == :out_of_stock ? 'Out of Stock' : 'On Cooldown'
        render Components::Medications::TakeAction.new(
          source: schedule,
          context: { person: person, current_user: current_user },
          amount: schedule.dosage.amount,
          button: {
            label: 'Take Now',
            variant: :success_outline,
            size: :sm,
            class: 'inline-block',
            testid: "take-medication-#{schedule.id}",
            form_class: 'inline-block'
          },
          state: {
            disabled: blocked_reason.present?,
            label: label
          }
        )
      end

      def blocked_reason
        @blocked_reason ||= MedicationStockSourceResolver.new(user: current_user, source: schedule).blocked_reason
      end

      def render_delete_button
        render Components::Dashboard::DeleteConfirmationDialog.new(
          schedule: schedule
        )
      end
    end
  end
end

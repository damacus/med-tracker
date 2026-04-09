# frozen_string_literal: true

module Components
  module Dashboard
    # Renders a schedule card for mobile view
    class ScheduleCard < Components::Base
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
        Card(class: 'p-4', id: "schedule_#{schedule.id}") do
          render_card_content
          render_card_actions
        end
      end

      private

      def render_card_content
        div(class: 'flex items-start justify-between gap-3') do
          div(class: 'flex-1 min-w-0') do
            render_header
            render_medication_info
            render_details
          end
        end
      end

      def render_header
        div(class: 'flex justify-between items-start w-full mb-2') do
          div(class: 'flex items-center gap-2') do
            render_person_avatar
            span(class: 'font-semibold text-foreground truncate') { person.name }
          end
          render Components::Shared::StockBadge.new(medication: schedule.medication)
        end
      end

      def render_person_avatar
        Avatar(size: :sm) do
          AvatarFallback { '👤' }
        end
      end

      def render_medication_info
        div(class: 'flex items-center gap-2 mb-3') do
          render_medication_icon
          span(class: 'font-medium text-foreground') { schedule.medication.name }
        end
      end

      def render_medication_icon
        div(class: 'w-8 h-8 rounded-lg flex items-center justify-center bg-success-light text-success flex-shrink-0') do
          render Icons::Pill.new(size: 16)
        end
      end

      def render_details
        div(class: 'grid grid-cols-2 gap-2 text-sm text-muted-foreground') do
          render_detail(t('dashboard.schedule_card.dosage'), format_dosage)
          render_detail(t('dashboard.schedule_card.remaining_supply'), format_quantity)
          render_detail(t('dashboard.schedule_card.frequency'), schedule.frequency || '—')
          render_detail(t('dashboard.schedule_card.ends'), format_end_date)
        end
      end

      def render_detail(label, value)
        div do
          span(class: 'text-muted-foreground') { "#{label}: " }
          span(class: 'font-medium') { value }
        end
      end

      def render_card_actions
        div(class: 'mt-4 flex flex-wrap gap-2') do
          render_take_now_button
          render_delete_button if can_delete?
        end
      end

      def render_take_now_button
        label = if blocked_reason == :out_of_stock
                  t('dashboard.person_schedule.out_of_stock')
                else
                  t('dashboard.person_schedule.on_cooldown')
                end
        render Components::Medications::TakeAction.new(
          source: schedule,
          context: { person: person, current_user: current_user },
          amount: schedule.dosage.amount,
          button: {
            label: t('dashboard.person_schedule.take_now'),
            variant: :success_outline,
            size: :md,
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

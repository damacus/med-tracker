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
        m3_card(variant: :elevated, class: 'p-6 bg-surface-container-low shadow-elevation-1', id: "schedule_#{schedule.id}") do
          render_card_content
          render_card_actions
        end
      end

      private

      def render_card_content
        div(class: 'flex items-start justify-between gap-4') do
          div(class: 'flex-1 min-w-0 space-y-4') do
            render_header
            render_medication_info
            render_details
          end
        end
      end

      def render_header
        div(class: 'flex justify-between items-center w-full') do
          div(class: 'flex items-center gap-3') do
            render_person_avatar
            m3_text(variant: :label_large, class: 'font-bold text-foreground truncate') { person.name }
          end
          render Components::Shared::StockBadge.new(medication: schedule.medication)
        end
      end

      def render_person_avatar
        div(class: 'w-8 h-8 rounded-full bg-secondary-container flex items-center justify-center text-on-secondary-container shadow-inner') do
          span(class: 'text-xs font-black') { person.name.first.upcase }
        end
      end

      def render_medication_info
        div(class: 'flex items-center gap-3') do
          render_medication_icon
          m3_heading(variant: :title_medium, class: 'font-bold text-foreground tracking-tight') { schedule.medication.name }
        end
      end

      def render_medication_icon
        div(class: 'w-10 h-10 rounded-xl flex items-center justify-center bg-success-container/30 text-on-success-container flex-shrink-0 shadow-inner') do
          render Icons::Pill.new(size: 20)
        end
      end

      def render_details
        div(class: 'grid grid-cols-2 gap-x-4 gap-y-2') do
          render_detail(t('dashboard.schedule_card.dosage'), format_dosage)
          render_detail(t('dashboard.schedule_card.remaining_supply'), format_quantity)
          render_detail(t('dashboard.schedule_card.frequency'), schedule.frequency || '—')
          render_detail(t('dashboard.schedule_card.ends'), format_end_date)
        end
      end

      def render_detail(label, value)
        div(class: 'space-y-0.5') do
          m3_text(variant: :label_small, class: 'uppercase tracking-widest text-on-surface-variant/60 font-black text-[9px]') { label }
          m3_text(variant: :body_small, class: 'font-bold text-on-surface-variant') { value }
        end
      end

      def render_card_actions
        div(class: 'mt-6 flex flex-wrap gap-3') do
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
          amount: schedule.dose_amount,
          button: {
            label: t('dashboard.person_schedule.take_now'),
            variant: :tonal,
            size: :md,
            class: 'inline-flex rounded-xl font-bold shadow-sm',
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
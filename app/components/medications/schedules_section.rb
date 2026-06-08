# frozen_string_literal: true

module Components
  module Medications
    class SchedulesSection < Components::Base
      attr_reader :medication, :schedules

      def initialize(medication:, schedules:)
        @medication = medication
        @schedules = schedules
        super()
      end

      def view_template
        div(class: 'space-y-4', data: { testid: 'medication-schedules-section' }) do
          render_header

          if schedules.to_a.any?
            div(class: 'space-y-3') do
              schedules.each { |schedule| render_schedule_row(schedule) }
            end
          else
            render_empty_state
          end
        end
      end

      private

      def render_header
        div(class: 'flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between') do
          div do
            m3_heading(variant: :title_large, level: 2, class: 'font-bold tracking-tight') do
              t('medications.show.schedules_heading')
            end
            m3_text(variant: :body_medium, class: 'text-on-surface-variant font-medium') do
              t('medications.show.schedules_description')
            end
          end

          render_add_schedule_action if can_update_medication?
        end
      end

      def render_add_schedule_action
        m3_link(
          href: schedules_workflow_path(medication_id: medication.id, return_to: medication_path(medication)),
          variant: :filled,
          size: :lg,
          class: 'w-full justify-center sm:w-auto',
          data: { testid: 'add-medication-schedule' }
        ) do
          render Icons::PlusCircle.new(size: 18, class: 'mr-2')
          span { t('medications.show.add_schedule_plan') }
        end
      end

      def render_schedule_row(schedule)
        m3_card(variant: :elevated, class: 'border border-border/60 p-5') do
          div(class: 'flex flex-col gap-4 lg:flex-row lg:items-center lg:justify-between') do
            div(class: 'min-w-0 space-y-3') do
              div(class: 'flex flex-wrap items-center gap-2') do
                m3_badge(variant: status_badge_variant(schedule)) { schedule_status(schedule) }
                m3_text(
                  variant: :label_medium,
                  class: 'font-black uppercase tracking-widest text-on-surface-variant'
                ) do
                  schedule.person.name
                end
              end

              div(class: 'space-y-1') do
                m3_text(variant: :title_medium, class: 'font-bold text-foreground') do
                  "#{dose_label(schedule)} - #{schedule.frequency}"
                end
                m3_text(variant: :body_medium, class: 'text-on-surface-variant font-medium') do
                  schedule_date_range(schedule)
                end
              end
            end

            render_schedule_actions(schedule) if can_update_medication?
          end
        end
      end

      def render_schedule_actions(schedule)
        div(class: 'flex items-center gap-2') do
          m3_link(
            href: edit_person_schedule_path(schedule.person, schedule, return_to: medication_path(medication)),
            variant: :outlined,
            size: :lg,
            class: 'h-11 w-11 p-0 justify-center',
            data: { turbo_frame: 'modal', testid: "edit-medication-schedule-#{schedule.id}" }
          ) do
            span(class: 'sr-only') { t('schedules.card.edit') }
            render Icons::Pencil.new(size: 18)
          end

          render_stop_schedule_action(schedule) if schedule.stopped_on.blank?
        end
      end

      def render_stop_schedule_action(schedule)
        form_with(
          url: stop_person_schedule_path(schedule.person, schedule),
          method: :patch,
          class: 'inline'
        ) do
          input(type: :hidden, name: 'return_to', value: medication_path(medication))
          m3_button(
            variant: :outlined,
            type: :submit,
            class: 'h-11 w-11 p-0 justify-center text-error hover:bg-error/5',
            data: { testid: "stop-medication-schedule-#{schedule.id}" }
          ) do
            span(class: 'sr-only') { t('schedules.card.stop') }
            render Icons::XCircle.new(size: 18)
          end
        end
      end

      def render_empty_state
        m3_card(variant: :filled, class: 'border border-dashed border-outline-variant/50 p-8 text-center') do
          m3_text(variant: :body_large, class: 'text-on-surface-variant font-medium') do
            t('medications.show.no_schedules')
          end
        end
      end

      def schedule_status(schedule)
        if schedule.stopped_on.present?
          t('medications.show.schedule_status.stopped')
        elsif schedule.active?
          t('medications.show.schedule_status.active')
        elsif schedule.start_date.present? && schedule.start_date > Time.zone.today
          t('medications.show.schedule_status.future')
        else
          t('medications.show.schedule_status.ended')
        end
      end

      def status_badge_variant(schedule)
        return :destructive if schedule.stopped_on.present?
        return :filled if schedule.active?
        return :tonal if schedule.start_date.present? && schedule.start_date > Time.zone.today

        :outlined
      end

      def schedule_date_range(schedule)
        t(
          'medications.show.schedule_dates',
          start_date: format_schedule_date(schedule.start_date),
          end_date: format_schedule_date(schedule.end_date)
        )
      end

      def format_schedule_date(value)
        value&.strftime('%Y-%m-%d') || t('schedules.index.ongoing')
      end

      def dose_label(schedule)
        DoseAmount.new(schedule.dose_amount, schedule.dose_unit).to_s
      end

      def can_update_medication?
        view_context.policy(medication).update?
      rescue NoMethodError
        true
      end
    end
  end
end

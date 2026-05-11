# frozen_string_literal: true

module Components
  module Dashboard
    class PersonTaskCard < Components::Base
      attr_reader :person, :routine_tasks, :as_needed_items, :current_user

      def initialize(person:, routine_tasks:, as_needed_items:, current_user: nil)
        @person = person
        @routine_tasks = routine_tasks
        @as_needed_items = as_needed_items
        @current_user = current_user
        super()
      end

      def view_template
        m3_card(
          variant: :elevated,
          class: 'border-none bg-surface-container-low p-5 rounded-[2rem] shadow-elevation-1'
        ) do
          render_header
          render_routine_tasks
          render_as_needed_items
        end
      end

      private

      def render_header
        div(class: 'flex items-start justify-between gap-4 pb-4') do
          div(class: 'min-w-0') do
            m3_heading(variant: :title_large, level: 3, class: 'font-black tracking-tight') { person.name }
            m3_text(variant: :body_medium, class: 'text-on-surface-variant font-medium') { routine_summary }
          end
          m3_badge(variant: remaining_routine_count.positive? ? :filled : :tonal,
                   class: 'shrink-0 px-3 py-1 text-[10px] font-black uppercase tracking-wider') do
            remaining_routine_count.positive? ? remaining_routine_count.to_s : t('dashboard.routine.done_short')
          end
        end
      end

      def render_routine_tasks
        if routine_tasks.any?
          div(class: 'space-y-3') do
            routine_tasks.each { |task| render_task_row(task, routine: true) }
          end
        else
          div(class: 'rounded-shape-xl border border-dashed border-outline-variant/70 p-4') do
            m3_text(variant: :body_medium, class: 'text-on-surface-variant italic') do
              t('dashboard.routine.empty')
            end
          end
        end
      end

      def render_as_needed_items
        return if as_needed_items.empty?

        details(class: 'mt-4 rounded-shape-xl border border-border bg-surface-container',
                data: { testid: 'dashboard-as-needed-person' }) do
          summary(class: as_needed_summary_classes) do
            t('dashboard.as_needed.title')
          end
          div(class: 'space-y-3 border-t border-border px-4 py-4') do
            as_needed_items.each { |item| render_task_row(item, routine: false) }
          end
        end
      end

      def render_task_row(row, routine:)
        div(
          id: timeline_dom_id(row[:source]),
          class: task_row_classes(row),
          data: { testid: routine ? 'dashboard-routine-task' : 'dashboard-as-needed-task' }
        ) do
          div(class: 'flex min-w-0 items-start gap-4') do
            div(class: 'w-16 shrink-0 pt-0.5 text-xs font-black uppercase tracking-widest text-on-surface-variant') do
              leading_label(row, routine:)
            end
            div(class: 'min-w-0') do
              m3_text(variant: :title_medium, class: 'font-bold tracking-tight break-words') do
                row[:source].medication.display_name
              end
              m3_text(variant: :body_small, class: 'text-on-surface-variant font-medium') { dose_label(row[:source]) }
            end
          end
          div(class: 'flex shrink-0 items-center gap-2 sm:justify-end') do
            render_action(row)
            render_status_badge(row)
          end
        end
      end

      def render_action(row)
        return unless %i[upcoming available].include?(row[:status])

        render Components::Medications::TakeAction.new(
          source: row[:source],
          context: { person: person, current_user: current_user },
          amount: row[:source].dose_amount,
          button: {
            label: take_label,
            variant: row[:status] == :available ? :outlined : :filled,
            size: :md,
            icon: Icons::Pill,
            testid: "take-dose-#{dose_id(row[:source])}",
            form_class: nil
          }
        )
      end

      def render_status_badge(row)
        m3_badge(variant: status_badge_variant(row[:status]),
                 class: 'px-3 py-1 text-[10px] font-black uppercase tracking-wider') do
          status_label(row)
        end
      end

      def routine_summary
        if remaining_routine_count.positive?
          t('dashboard.routine.left_today', count: remaining_routine_count)
        else
          t('dashboard.routine.done_today')
        end
      end

      def remaining_routine_count
        @remaining_routine_count ||= routine_tasks.count { |task| task[:status] != :taken }
      end

      def row_classes(row)
        case row[:status]
        when :taken then 'border-success/30 bg-success-container/30'
        when :upcoming, :available then 'border-primary/30 bg-surface-container-high'
        when :cooldown, :max_reached then 'border-warning/30 bg-warning-container/30'
        when :out_of_stock then 'border-error/30 bg-error-container/30'
        else 'border-border bg-surface-container-high'
        end
      end

      def leading_label(row, routine:)
        return row[:taken_at].strftime('%H:%M') if row[:status] == :taken && row[:taken_at]
        return row[:scheduled_at].strftime('%H:%M') if routine && row[:scheduled_at]

        routine ? t('dashboard.routine.anytime') : ''
      end

      def as_needed_summary_classes
        'cursor-pointer px-4 py-3 text-sm font-black uppercase tracking-widest text-on-surface-variant'
      end

      def task_row_classes(row)
        [
          'flex flex-col gap-3 rounded-shape-xl border p-4 sm:flex-row sm:items-center sm:justify-between',
          row_classes(row)
        ].join(' ')
      end

      def status_label(row)
        case row[:status]
        when :taken then t('dashboard.statuses.taken')
        when :upcoming then t('dashboard.statuses.upcoming')
        when :available then t('dashboard.statuses.available_now')
        when :cooldown then cooldown_label(row)
        when :max_reached then t('dashboard.statuses.max_reached')
        when :out_of_stock then t('dashboard.statuses.out_of_stock')
        else t("dashboard.statuses.#{row[:status]}")
        end
      end

      def cooldown_label(row)
        return t('dashboard.statuses.available_at', time: row[:scheduled_at].strftime('%H:%M')) if row[:scheduled_at]
        return t('dashboard.statuses.cooldown') unless row[:source].respond_to?(:countdown_display)

        "#{t('dashboard.statuses.cooldown')} (#{row[:source].countdown_display})"
      end

      def status_badge_variant(status)
        case status
        when :taken then :tonal
        when :upcoming, :available then :filled
        when :out_of_stock then :destructive
        else :outlined
        end
      end

      def dose_label(source)
        source.dose_display.presence || DoseAmount.new(source.dose_amount, source.dose_unit).to_s
      end

      def own_dose?
        return true if current_user.nil?

        current_user.person == person
      end

      def take_label
        own_dose? ? t('person_medications.card.take') : t('person_medications.card.give')
      end

      def dose_id(source)
        "#{source.class.name.downcase}_#{source.id}"
      end

      def timeline_dom_id(source)
        "timeline_#{source.class.name.underscore}_#{source.id}"
      end
    end
  end
end

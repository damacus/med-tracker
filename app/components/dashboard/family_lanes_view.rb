# frozen_string_literal: true

module Components
  module Dashboard
    class FamilyLanesView < ExperimentalBaseView
      GROUPINGS = %w[person time].freeze

      attr_reader :grouping

      def initialize(presenter:, grouping: 'person')
        @grouping = GROUPINGS.include?(grouping) ? grouping : 'person'
        super(presenter: presenter)
      end

      def view_template
        div(
          class: 'container mx-auto max-w-6xl px-4 py-6',
          data: { testid: 'dashboard-variant-family-lanes' }
        ) do
          render_header
          render_family_toolbar
          grouping == 'time' ? render_time_grouping : render_person_grouping
          render_stock_summary(compact: true)
          render_version_footer
        end
      end

      private

      def render_family_toolbar
        div(class: 'mb-6 flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between') do
          m3_heading(variant: :title_large, level: 2, class: 'font-black tracking-tight') do
            t('dashboard.variants.family_lanes.title')
          end
          nav(
            class: 'inline-flex w-full rounded-shape-full border border-outline-variant ' \
                   'bg-surface-container-low p-1 sm:w-auto',
            aria: { label: t('dashboard.variants.family_lanes.grouping_label') }
          ) do
            render_grouping_link('person', Icons::User)
            render_grouping_link('time', Icons::Clock)
          end
        end
      end

      def render_grouping_link(value, icon)
        selected = grouping == value
        m3_link(
          href: grouping_path(value),
          variant: selected ? :filled : :text,
          size: :md,
          class: "flex-1 gap-2 rounded-shape-full px-5 font-black sm:flex-none #{grouping_link_classes(selected)}",
          aria: selected ? { current: 'page' } : {}
        ) do
          render icon.new(size: 18)
          span { t("dashboard.variants.family_lanes.by_#{value}") }
        end
      end

      def grouping_link_classes(selected)
        selected ? 'shadow-elevation-1' : 'text-on-surface-variant hover:bg-surface-container-high'
      end

      def render_person_grouping
        if people.any?
          div(class: 'mb-6 grid grid-cols-1 gap-5 lg:grid-cols-2') do
            people.each { |person| render_person_lane(person) }
          end
        else
          render_empty_dashboard
        end
      end

      def render_person_lane(person)
        entries = entries_for_person(person)
        m3_card(
          variant: :outlined,
          class: 'min-h-[28rem] overflow-hidden rounded-[2rem] border-border bg-surface-container-low'
        ) do
          div(class: 'flex items-center justify-between gap-4 border-b border-border px-6 py-5') do
            render_person_identity(person, size: :md)
            m3_text(variant: :body_medium, class: 'shrink-0 text-on-surface-variant') do
              t('dashboard.variants.family_lanes.task_count', count: entries.count { |entry| entry[:kind] == :task })
            end
          end
          if entries.any?
            div(class: 'divide-y divide-border/70 px-5') do
              entries.each { |entry| render_lane_entry(entry) }
            end
          else
            render_lane_empty_state
          end
        end
      end

      def render_lane_entry(entry)
        div(class: 'py-5') do
          div(class: 'mb-3 flex items-center justify-between gap-3') do
            m3_text(variant: :label_large, class: status_heading_classes(entry)) { status_label_for(entry) }
            span(class: 'font-black tabular-nums') { time_label_for(entry) }
          end
          div(class: 'flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between') do
            div(class: 'flex min-w-0 items-center gap-4') do
              span(
                class: 'inline-flex h-11 w-11 shrink-0 items-center justify-center ' \
                       'rounded-shape-full bg-primary-container'
              ) do
                render_medication_icon(entry, class_name: 'text-on-primary-container')
              end
              div(class: 'min-w-0') do
                m3_heading(variant: :title_medium, level: 3, class: 'truncate font-black') do
                  medication_for(entry).display_name
                end
                m3_text(variant: :body_medium, class: 'text-on-surface-variant') { dose_label_for(entry) }
              end
            end
            render_lane_action(entry)
          end
        end
      end

      def render_lane_action(entry)
        actionable = entry[:kind] == :task && ACTIONABLE_STATUSES.include?(entry[:status])
        return render_status_badge_for(entry) unless actionable

        render_task_action(
          entry,
          label: take_label_for(entry),
          variant: due_now?(entry) ? :filled : :outlined,
          class_name: 'w-full font-black sm:w-auto'
        )
      end

      def render_lane_empty_state
        div(class: 'flex min-h-72 flex-col items-center justify-center p-8 text-center') do
          span(
            class: 'mb-4 inline-flex h-14 w-14 items-center justify-center rounded-shape-full ' \
                   'bg-surface-container-high'
          ) do
            render Icons::Calendar.new(size: 26, class: 'text-on-surface-variant')
          end
          m3_heading(variant: :title_medium, level: 3, class: 'font-black') do
            t('dashboard.variants.no_more_tasks')
          end
          m3_text(variant: :body_medium, class: 'mt-2 text-on-surface-variant') do
            t('dashboard.variants.family_lanes.all_caught_up')
          end
        end
      end

      def render_time_grouping
        section(class: 'mb-6 space-y-5', data: { testid: 'dashboard-family-time' }) do
          if timeline_entries.any?
            grouped_timeline_entries.each do |period, entries|
              m3_card(variant: :outlined, class: 'rounded-[2rem] border-border bg-surface-container-low p-5') do
                render_period_heading(period)
                div(class: 'divide-y divide-border/70') do
                  entries.each { |entry| render_time_entry(entry) }
                end
              end
            end
          else
            render_empty_dashboard
          end
        end
      end

      def render_time_entry(entry)
        div(class: 'grid grid-cols-[3.5rem_1fr_auto] items-center gap-3 py-4 sm:grid-cols-[4rem_1fr_1fr_auto]') do
          span(class: 'font-black tabular-nums text-primary') { time_label_for(entry) }
          render_person_identity(entry[:person])
          div(class: 'hidden min-w-0 sm:block') do
            m3_text(variant: :body_medium, class: 'truncate font-black') { medication_for(entry).display_name }
            m3_text(variant: :body_small, class: 'text-on-surface-variant') { dose_label_for(entry) }
          end
          render_status_badge_for(entry)
        end
      end

      def status_heading_classes(entry)
        due_now?(entry) ? 'font-black text-warning' : 'font-black text-primary'
      end
    end
  end
end

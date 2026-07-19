# frozen_string_literal: true

module Components
  module Dashboard
    class TimeFirstView < ExperimentalBaseView
      def view_template
        div(
          class: 'container mx-auto max-w-6xl px-4 py-6',
          data: { testid: 'dashboard-variant-time-first' }
        ) do
          render_header
          render_person_selector
          if next_task
            render_dashboard_grid
          else
            render_empty_dashboard
          end
          render_version_footer
        end
      end

      private

      def render_dashboard_grid
        div(class: 'grid grid-cols-1 items-start gap-6 lg:grid-cols-3') do
          main(class: 'space-y-7 lg:col-span-2') do
            render_next_up
            render_chronological_timeline
          end
          aside(class: 'space-y-5') do
            render_later_today
            render_stock_summary(stacked: true)
          end
        end
      end

      def render_next_up
        section(class: 'space-y-3', aria: { labelledby: 'time-first-next-up' }) do
          m3_heading(id: 'time-first-next-up', variant: :title_large, level: 2, class: 'font-black tracking-tight') do
            t('dashboard.variants.time_first.next_up')
          end
          m3_card(
            variant: :elevated,
            class: 'rounded-[2rem] border border-primary/20 bg-surface-container-low p-6 shadow-elevation-2 sm:p-8'
          ) do
            div(class: 'grid gap-6 sm:grid-cols-[1fr_auto] sm:items-center') do
              div(class: 'min-w-0') do
                render_person_identity(next_task[:person], size: :md)
                div(class: 'mt-6 flex items-start gap-4') do
                  span(
                    class: 'inline-flex h-12 w-12 shrink-0 items-center justify-center ' \
                           'rounded-shape-xl bg-primary-container'
                  ) do
                    render_medication_icon(next_task, size: 26, class_name: 'text-on-primary-container')
                  end
                  div(class: 'min-w-0') do
                    m3_heading(variant: :headline_small, level: 3, class: 'font-black tracking-tight') do
                      medication_for(next_task).display_name
                    end
                    m3_text(variant: :title_medium, class: 'mt-1 text-on-surface-variant') do
                      dose_label_for(next_task)
                    end
                  end
                end
              end
              div(class: 'min-w-52 border-t border-border pt-5 sm:border-l sm:border-t-0 sm:pl-7 sm:pt-0') do
                m3_text(variant: :label_large, class: 'text-on-surface-variant') do
                  t('dashboard.variants.due_at')
                end
                m3_text(variant: :display_small, class: 'my-2 font-black tabular-nums text-primary') do
                  time_label_for(next_task)
                end
                render_task_action(
                  next_task,
                  label: take_label_for(next_task),
                  size: :lg,
                  class_name: 'w-full font-black shadow-elevation-1'
                )
              end
            end
          end
        end
      end

      def render_chronological_timeline
        section(class: 'space-y-5', aria: { labelledby: 'time-first-timeline' }) do
          m3_heading(id: 'time-first-timeline', variant: :title_large, level: 2, class: 'sr-only') do
            t('dashboard.variants.time_first.timeline')
          end
          grouped_timeline_entries.each do |period, entries|
            div(class: 'space-y-1') do
              render_period_heading(period)
              entries.each { |entry| render_timeline_row(entry) }
            end
          end
        end
      end

      def render_timeline_row(entry)
        div(
          class: 'grid grid-cols-[3.5rem_1fr_auto] items-center gap-3 border-b border-border/60 px-1 py-4 ' \
                 'sm:grid-cols-[4rem_1.2fr_1fr_auto]'
        ) do
          span(class: 'text-sm font-black tabular-nums text-on-surface-variant') { time_label_for(entry) }
          render_person_identity(entry[:person])
          div(class: 'hidden min-w-0 sm:block') do
            m3_text(variant: :body_medium, class: 'truncate font-bold') { medication_for(entry).display_name }
            m3_text(variant: :body_small, class: 'text-on-surface-variant') { dose_label_for(entry) }
          end
          render_status_badge_for(entry)
        end
      end

      def render_later_today
        m3_card(variant: :outlined, class: 'rounded-[2rem] border-border bg-surface-container-low p-6') do
          div(class: 'mb-4 flex items-center justify-between gap-3') do
            m3_heading(variant: :title_large, level: 2, class: 'font-black') do
              t('dashboard.variants.time_first.later_today')
            end
            m3_badge(variant: :tonal, class: 'px-3 py-1 font-black') { following_tasks.length.to_s }
          end
          if following_tasks.any?
            div(class: 'divide-y divide-border/70') do
              following_tasks(limit: 4).each { |row| render_later_row(row) }
            end
          else
            m3_text(variant: :body_medium, class: 'py-4 text-on-surface-variant') do
              t('dashboard.variants.no_more_tasks')
            end
          end
        end
      end

      def render_later_row(row)
        div(class: 'grid grid-cols-[4.5rem_minmax(0,1fr)] items-start gap-3 py-4') do
          span(class: 'text-sm font-black tabular-nums text-primary') { time_label_for(row) }
          div(class: 'min-w-0') do
            div(class: 'flex items-start justify-between gap-2') do
              m3_text(variant: :body_medium, class: 'truncate font-black') { row[:person].name }
              m3_text(variant: :body_small, class: 'shrink-0 text-on-surface-variant') { dose_label_for(row) }
            end
            m3_text(variant: :body_small, class: 'truncate text-on-surface-variant') do
              medication_for(row).display_name
            end
          end
        end
      end
    end
  end
end

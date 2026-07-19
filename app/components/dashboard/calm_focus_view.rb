# frozen_string_literal: true

module Components
  module Dashboard
    class CalmFocusView < ExperimentalBaseView
      def view_template
        div(
          class: 'container mx-auto max-w-6xl px-4 py-6',
          data: { testid: 'dashboard-variant-calm-focus' }
        ) do
          render_header
          render_person_selector
          next_task ? render_focus_layout : render_empty_dashboard
          render_version_footer
        end
      end

      private

      def render_focus_layout
        div(class: 'grid grid-cols-1 items-start gap-5 lg:grid-cols-5') do
          div(class: 'space-y-4 lg:col-span-3') do
            render_attention_card
            render_completed_today
            render_stock_notice
          end
          render_after_this
        end
      end

      def render_attention_card
        m3_card(
          variant: :elevated,
          class: 'rounded-[2rem] border border-primary/20 bg-surface-container-low p-6 shadow-elevation-2 sm:p-8'
        ) do
          div(class: 'flex items-center gap-4') do
            span(
              class: 'inline-flex h-12 w-12 shrink-0 items-center justify-center rounded-shape-full ' \
                     'bg-primary text-on-primary'
            ) do
              render Icons::Bell.new(size: 24)
            end
            m3_heading(variant: :headline_small, level: 2, class: 'font-black tracking-tight') do
              t('dashboard.variants.calm_focus.needs_attention')
            end
          end
          div(class: 'mt-7 border-b border-border pb-6') do
            render_person_identity(next_task[:person], size: :md)
          end
          div(class: 'grid gap-6 py-7 sm:grid-cols-[1fr_auto] sm:items-center') do
            div(class: 'flex min-w-0 items-center gap-4') do
              span(
                class: 'inline-flex h-16 w-16 shrink-0 items-center justify-center ' \
                       'rounded-shape-xl bg-primary-container'
              ) do
                render_medication_icon(next_task, size: 32, class_name: 'text-on-primary-container')
              end
              div(class: 'min-w-0') do
                m3_heading(variant: :headline_small, level: 3, class: 'truncate font-black') do
                  medication_for(next_task).display_name
                end
                m3_text(variant: :title_medium, class: 'text-on-surface-variant') { dose_label_for(next_task) }
              end
            end
            div(class: 'flex items-center gap-3 sm:border-l sm:border-border sm:pl-6') do
              span(
                class: 'inline-flex h-12 w-12 items-center justify-center rounded-shape-full ' \
                       'border border-primary text-primary'
              ) do
                render Icons::Clock.new(size: 22)
              end
              div do
                m3_text(variant: :headline_small, class: 'font-black tabular-nums') { time_label_for(next_task) }
                m3_text(variant: :label_large, class: 'font-black text-primary') { status_label_for(next_task) }
              end
            end
          end
          render_task_action(
            next_task,
            label: t('dashboard.variants.calm_focus.review_and_record'),
            size: :lg,
            class_name: 'w-full font-black shadow-elevation-2'
          )
          m3_text(variant: :body_medium, class: 'mt-4 text-center text-on-surface-variant') do
            t('dashboard.variants.calm_focus.confirmation_hint')
          end
        end
      end

      def render_after_this
        m3_card(
          variant: :outlined,
          class: 'rounded-[2rem] border-border bg-surface-container-low p-6 lg:col-span-2'
        ) do
          m3_heading(variant: :headline_small, level: 2, class: 'mb-5 font-black tracking-tight') do
            t('dashboard.variants.calm_focus.after_this')
          end
          if following_tasks.any?
            div(class: 'divide-y divide-border/70') do
              following_tasks(limit: 4).each { |row| render_following_task(row) }
            end
          else
            m3_text(variant: :body_medium, class: 'py-8 text-center text-on-surface-variant') do
              t('dashboard.variants.no_more_tasks')
            end
          end
        end
      end

      def render_following_task(row)
        div(class: 'grid grid-cols-[3.5rem_1fr] gap-4 py-5') do
          span(class: 'font-black tabular-nums text-primary') { time_label_for(row) }
          div(class: 'min-w-0') do
            render_person_identity(row[:person])
            m3_text(variant: :body_medium, class: 'mt-2 truncate font-black') do
              medication_for(row).display_name
            end
            m3_text(variant: :body_small, class: 'text-on-surface-variant') { dose_label_for(row) }
          end
        end
      end

      def render_completed_today
        details(
          class: 'rounded-[1.5rem] border border-border bg-surface-container-low',
          data: { testid: 'dashboard-calm-completed' }
        ) do
          summary(class: 'flex cursor-pointer items-center justify-between gap-4 px-5 py-4') do
            div(class: 'flex items-center gap-3') do
              span(
                class: 'inline-flex h-10 w-10 items-center justify-center rounded-shape-full ' \
                       'bg-success-container text-on-success-container'
              ) do
                render Icons::CheckCircle.new(size: 22)
              end
              m3_heading(variant: :title_medium, level: 2, class: 'font-black') do
                t('dashboard.variants.calm_focus.completed_today')
              end
            end
            m3_text(variant: :body_medium, class: 'font-black text-on-surface-variant') do
              t('dashboard.variants.calm_focus.completed_count', count: completed_entries.count)
            end
          end
          div(class: 'divide-y divide-border border-t border-border px-5') do
            if completed_entries.any?
              completed_entries.each { |entry| render_completed_entry(entry) }
            else
              m3_text(variant: :body_medium, class: 'block py-4 text-on-surface-variant') do
                t('dashboard.variants.calm_focus.none_completed')
              end
            end
          end
        end
      end

      def render_completed_entry(entry)
        div(class: 'flex items-center justify-between gap-4 py-4') do
          div(class: 'min-w-0') do
            m3_text(variant: :body_medium, class: 'truncate font-black') { medication_for(entry).display_name }
            m3_text(variant: :body_small, class: 'text-on-surface-variant') { entry[:person].name }
          end
          span(class: 'shrink-0 text-sm font-black tabular-nums text-success') { time_label_for(entry) }
        end
      end

      def render_stock_notice
        render_stock_summary(compact: true)
      end
    end
  end
end

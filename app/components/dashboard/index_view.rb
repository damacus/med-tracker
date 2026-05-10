# frozen_string_literal: true

module Components
  module Dashboard
    # Dashboard index view component that renders the main dashboard page
    class IndexView < Components::Base
      FILTERS = %w[all needs_action upcoming taken].freeze

      attr_reader :presenter, :filter

      def initialize(presenter:, filter: 'all')
        @presenter = presenter
        @filter = FILTERS.include?(filter.to_s) ? filter.to_s : 'all'
        super()
      end

      def view_template
        div(class: 'container mx-auto max-w-3xl px-4 pb-24 pt-4 md:py-8', data: { testid: 'dashboard' }) do
          render_header
          render_mobile_title
          render_daily_summary
          render_filter_strip
          render_timeline_section
          render_version_footer
        end
      end

      private

      delegate :current_user, :doses, :next_dose_time, :as_needed_by_person, to: :presenter

      def render_header
        div(class: 'mb-8 hidden flex-col justify-between gap-6 md:flex md:flex-row md:items-end') do
          div do
            m3_text(variant: :label_large, class: 'uppercase tracking-[0.2em] mb-1 block font-black opacity-40') do
              Time.current.strftime('%A, %b %d')
            end
            m3_heading(variant: :display_small, level: 1, class: 'font-black tracking-tight text-foreground') do
              if current_user.person&.name.present?
                t(greeting_key, name: current_user.person.name.split.first)
              else
                t('dashboard.title')
              end
            end
          end
          div(class: 'hidden gap-3 md:flex') do
            if can_create_person?
              m3_link(
                href: new_person_path,
                variant: :outlined,
                size: :lg,
                class: 'rounded-xl font-bold bg-surface-container-low transition-all'
              ) do
                t('dashboard.quick_actions.add_person')
              end
            end
            m3_link(
              href: add_medication_path,
              variant: :filled,
              size: :lg,
              class: 'rounded-xl font-bold px-8 shadow-lg shadow-primary/20 transition-all',
              data: { turbo_frame: 'modal' }
            ) do
              render Icons::PlusCircle.new(size: 20, class: 'mr-2')
              span { t('dashboard.quick_actions.add_medication') }
            end
          end
        end
      end

      def render_mobile_title
        div(class: 'mb-4 flex items-end justify-between gap-4 md:hidden') do
          div(class: 'min-w-0') do
            m3_heading(
              variant: :headline_medium,
              level: 1,
              class: 'text-3xl font-black leading-tight tracking-tight text-foreground'
            ) do
              t('dashboard.todays_medicines')
            end
            m3_text(variant: :body_medium, class: 'mt-2 font-medium text-on-surface-variant') do
              Time.current.strftime('%A, %d %B')
            end
          end
          div(
            class: 'flex h-11 w-11 shrink-0 items-center justify-center rounded-shape-xl border ' \
                   'border-primary/20 bg-primary-container/40 text-primary shadow-elevation-1'
          ) do
            render Icons::Calendar.new(size: 20)
          end
        end
      end

      def render_daily_summary
        m3_card(
          variant: :filled,
          class: 'mb-4 overflow-hidden rounded-shape-xl border border-primary/15 bg-gradient-to-br ' \
                 'from-emerald-50 via-white to-sky-50 shadow-elevation-2 ' \
                 'dark:from-emerald-950/30 dark:via-surface-container-low dark:to-sky-950/20',
          data: { testid: 'dashboard-daily-summary' }
        ) do
          div(class: 'p-5') do
            div(class: 'flex items-start justify-between gap-5') do
              div(class: 'min-w-0 flex-1') do
                m3_text(variant: :title_medium, class: 'font-black tracking-tight text-foreground') do
                  t('dashboard.summary.today_progress', completed: completed_dose_count, total: total_dose_count)
                end
                m3_text(variant: :body_small, class: 'mt-1 text-on-surface-variant') do
                  summary_support_text
                end
              end

              div(class: 'shrink-0 text-right') do
                div(class: 'text-2xl font-black leading-none text-primary') { "#{completion_percentage}%" }
                m3_text(variant: :label_small, class: 'text-on-surface-variant') { t('dashboard.summary.completed') }
              end
            end

            div(class: 'mt-4 flex gap-1.5', aria: { hidden: 'true' }) do
              progress_segments.times do |index|
                div(class: progress_segment_class(index))
              end
            end

            div(class: 'mt-4 flex flex-wrap items-center gap-2') do
              div(
                class: 'inline-flex items-center gap-2 rounded-shape-full bg-white/80 px-3 py-2 ' \
                       'text-primary shadow-elevation-1 dark:bg-surface-container'
              ) do
                render Icons::Clock.new(size: 17)
                m3_text(variant: :label_large, class: 'font-black') do
                  t('dashboard.summary.next_due', time: next_due_value)
                end
              end
              div(
                class: 'inline-flex items-center gap-2 rounded-shape-full bg-teal-100/80 px-3 py-2 ' \
                       'text-teal-700 dark:bg-teal-900/30 dark:text-teal-200'
              ) do
                render Icons::CheckCircle.new(size: 17)
                m3_text(variant: :label_large, class: 'font-black') do
                  t('dashboard.summary.remaining', count: remaining_dose_count)
                end
              end
            end
          end
        end
      end

      def render_filter_strip
        nav(
          class: 'no-scrollbar mb-5 overflow-x-auto pb-1',
          aria: { label: t('dashboard.filters.label') },
          data: { testid: 'dashboard-filter-strip' }
        ) do
          div(class: 'flex min-w-max gap-2') do
            filter_options.each do |option|
              a(
                href: dashboard_path(filter: option[:value]),
                class: filter_chip_class(option[:value]),
                aria: { current: option[:value] == filter ? 'page' : nil }
              ) do
                render option[:icon].new(size: 17, class: 'shrink-0')
                span { option[:label] }
                span(class: filter_count_class(option[:value])) { option[:count].to_s }
              end
            end
          end
        end
      end

      def filter_options
        [
          {
            value: 'all',
            label: t('dashboard.filters.all'),
            count: medicine_rows.size,
            icon: Icons::Home
          },
          {
            value: 'needs_action',
            label: t('dashboard.filters.needs_action'),
            count: medicine_rows.count { |dose| action_needed?(dose) },
            icon: Icons::CheckCircle
          },
          {
            value: 'upcoming',
            label: t('dashboard.filters.upcoming'),
            count: medicine_rows.count { |dose| upcoming_or_blocked?(dose) },
            icon: Icons::Clock
          },
          {
            value: 'taken',
            label: t('dashboard.filters.taken'),
            count: medicine_rows.count { |dose| dose[:status] == :taken },
            icon: Icons::Check
          }
        ]
      end

      def render_timeline_section
        div(class: 'space-y-4') do
          div(class: 'flex items-center justify-between px-1') do
            m3_heading(variant: :title_large, level: 2, class: 'font-black tracking-tight') do
              t('dashboard.todays_medicines')
            end
            m3_text(variant: :label_large, class: 'font-black text-on-surface-variant') do
              t('dashboard.filtered_count', count: filtered_doses.size)
            end
          end

          if filtered_doses.any?
            div(class: 'space-y-3', data: { testid: 'dashboard-medicine-list' }) do
              filtered_doses.each do |dose|
                render Components::Dashboard::TimelineItem.new(dose: dose, current_user: current_user)
              end
            end
          else
            m3_card(variant: :filled,
                    class: 'rounded-shape-xl border-2 border-dashed border-outline-variant/50 ' \
                           'bg-surface-container-low p-8 text-center',
                    data: { testid: 'dashboard-medicine-list' }) do
              m3_text(variant: :body_large, class: 'text-on-surface-variant font-medium') do
                t(filtered_empty_key)
              end
            end
          end
        end
      end

      def filtered_empty_key
        filter == 'all' ? 'dashboard.empty_state' : 'dashboard.empty_filtered_state'
      end

      def filtered_doses
        @filtered_doses ||= begin
          predicate = filtered_dose_predicate
          predicate ? medicine_rows.select { |dose| send(predicate, dose) } : medicine_rows
        end
      end

      def medicine_rows
        @medicine_rows ||= (doses + as_needed_rows).sort_by do |dose|
          [dose[:scheduled_at] || Time.current.end_of_day, dose[:source].id]
        end
      end

      def as_needed_rows
        as_needed_by_person.values.flatten
      end

      def filtered_dose_predicate
        {
          'needs_action' => :action_needed?,
          'upcoming' => :upcoming_or_blocked?,
          'taken' => :taken?
        }[filter]
      end

      def action_needed?(dose)
        %i[upcoming available].include?(dose[:status])
      end

      def upcoming_or_blocked?(dose)
        %i[upcoming cooldown max_reached out_of_stock].include?(dose[:status])
      end

      def taken?(dose)
        dose[:status] == :taken
      end

      def filter_chip_class(value)
        base = 'inline-flex h-12 items-center gap-2 rounded-shape-full border px-4 text-sm font-black ' \
               'transition-all whitespace-nowrap'
        if value == filter
          "#{base} border-primary bg-primary text-on-primary shadow-elevation-2"
        else
          "#{base} border-outline-variant bg-surface-container-low text-on-surface-variant shadow-elevation-1 " \
            'hover:border-primary/60 hover:text-primary'
        end
      end

      def filter_count_class(value)
        if value == filter
          'ml-1 rounded-shape-full bg-white/20 px-2 py-0.5 text-xs text-on-primary'
        else
          'ml-1 rounded-shape-full bg-surface-container-high px-2 py-0.5 text-xs text-on-surface-variant'
        end
      end

      def progress_segments
        total_dose_count.clamp(1, 8)
      end

      def progress_segment_class(index)
        base = 'h-2 flex-1 rounded-shape-full transition-colors'
        return "#{base} bg-primary shadow-sm shadow-primary/30" if index < completed_progress_segments

        "#{base} bg-white/70 dark:bg-surface-container-high"
      end

      def completed_progress_segments
        return 0 if total_dose_count.zero?

        ((completed_dose_count.to_f / total_dose_count) * progress_segments).round
      end

      def remaining_dose_count
        [total_dose_count - completed_dose_count, 0].max
      end

      def greeting_key
        case Time.current.hour
        when 5..11 then 'dashboard.greeting_morning'
        when 12..17 then 'dashboard.greeting_afternoon'
        else 'dashboard.greeting_evening'
        end
      end

      def can_create_person?
        return false unless view_context.respond_to?(:policy)

        view_context.policy(Person.new).new?
      end

      def completed_dose_count
        @completed_dose_count ||= doses.count { |dose| dose[:status] == :taken }
      end

      def total_dose_count
        @total_dose_count ||= doses.size
      end

      def completion_percentage
        return 0 if total_dose_count.zero?

        ((completed_dose_count.to_f / total_dose_count) * 100).round.clamp(0, 100)
      end

      def next_due_value
        time = next_dose_time
        time ? time.strftime('%l:%M %p').strip : t('dashboard.summary.no_next_due')
      end

      def summary_support_text
        return t('dashboard.summary.all_done') if total_dose_count.positive? && completed_dose_count == total_dose_count

        t('dashboard.summary.keep_it_up')
      end

      def render_version_footer
        div(class: 'mt-10 border-t border-outline-variant/30 pt-6 text-center') do
          span(class: 'text-[10px] text-on-surface-variant/50 font-mono font-bold uppercase tracking-widest') do
            "v#{app_version}"
          end
        end
      end

      def app_version
        ENV.fetch('APP_VERSION', MedTracker::VERSION)
      end
    end
  end
end

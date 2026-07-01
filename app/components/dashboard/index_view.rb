# frozen_string_literal: true

module Components
  module Dashboard
    # Dashboard index view component that renders the main dashboard page
    class IndexView < Components::Base
      DIRECT_PERSON_OPTION_LIMIT = 5

      attr_reader :presenter

      def initialize(presenter:)
        @presenter = presenter
        super()
      end

      def view_template
        div(class: 'container mx-auto max-w-6xl px-4 py-6', data: { testid: 'dashboard' }) do
          render_header
          render_person_selector
          render_stats_section

          div(class: 'grid grid-cols-1 lg:grid-cols-3 gap-8 items-start') do
            div(class: 'lg:col-span-2') do
              render_timeline_section
            end
            div(class: 'space-y-8') do
              render_supply_levels
            end
            div(class: 'lg:col-span-2') do
              render_today_dose_history
              render_health_insights
            end
          end
          render_version_footer
        end
      end

      private

      delegate :people, :active_schedules, :upcoming_schedules,
               :current_user, :doses, :next_due_value, :due_now_count, :tasks_left_count,
               :routine_tasks_by_person, :as_needed_by_person, :today_takes_by_person,
               :dashboard_person_options, to: :presenter

      def render_header
        div(class: 'flex flex-col md:flex-row md:items-end justify-between gap-6 mb-8') do
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
                class: 'font-bold bg-surface-container-low transition-all'
              ) do
                t('dashboard.quick_actions.add_person')
              end
            end
            m3_link(
              href: add_medication_path,
              variant: :filled,
              size: :lg,
              class: 'font-bold shadow-elevation-2 transition-all',
              data: { turbo_frame: 'modal' }
            ) do
              render Icons::PlusCircle.new(size: 20, class: 'mr-2')
              span { t('dashboard.quick_actions.add_medication') }
            end
          end
        end
      end

      def render_person_selector
        return if dashboard_person_options.empty?

        nav(
          aria: { label: t('dashboard.person_selector.label') },
          class: 'max-w-full mb-6',
          data: { testid: 'dashboard-person-selector' }
        ) do
          div(class: 'max-w-full rounded-2xl bg-surface-container-low p-1') do
            div(class: 'flex min-w-0 items-center gap-2 sm:hidden') do
              render_mobile_person_selector_current
              render_mobile_person_selector_overflow
            end
            div(class: 'hidden max-w-full flex-wrap items-center gap-2 sm:flex') do
              direct_person_options.each do |option|
                render_person_selector_option(option)
              end
              render_person_selector_overflow
            end
          end
        end
      end

      def render_person_selector_option(option)
        selected = option.fetch(:selected)
        link_class = if selected
                       'bg-primary text-on-primary shadow-elevation-1'
                     else
                       'text-on-surface-variant hover:bg-surface-container-high hover:text-on-surface'
                     end

        a(
          href: dashboard_path(dashboard_person_id: option.fetch(:id)),
          class: "inline-flex min-h-12 items-center gap-2 rounded-xl px-3 py-2 text-sm font-bold #{link_class}",
          aria: selected ? { current: 'true' } : {},
          data: { testid: 'dashboard-person-option' }
        ) do
          render_person_selector_avatar(option)
          span(class: 'whitespace-nowrap') { option.fetch(:label) }
        end
      end

      def render_person_selector_avatar(option)
        person = option.fetch(:person)
        return render Components::Shared::PersonAvatar.new(person: person, size: :sm) if person

        span(
          class: 'inline-flex h-8 w-8 shrink-0 items-center justify-center rounded-shape-full ' \
                 'bg-secondary-container text-xs font-black text-on-secondary-container',
          data: { testid: 'person-avatar' },
          aria: { label: option.fetch(:label) }
        ) { option.fetch(:initials) }
      end

      def render_mobile_person_selector_current
        option = selected_person_selector_option || direct_person_options.first
        return unless option

        div(
          class: 'inline-flex min-h-12 min-w-0 flex-1 items-center gap-2 rounded-xl bg-primary px-3 py-2 ' \
                 'text-sm font-bold text-on-primary shadow-elevation-1',
          aria: { current: 'true' },
          data: { testid: 'dashboard-person-mobile-current' }
        ) do
          render_person_selector_avatar(option)
          span(class: 'truncate') { option.fetch(:label) }
        end
      end

      def render_mobile_person_selector_overflow
        return if mobile_overflow_selector_options.empty?

        render_person_selector_dropdown(
          options: mobile_overflow_selector_options,
          testid: 'dashboard-person-mobile-overflow',
          trigger_class: 'w-36 shrink-0',
          content_class: 'w-64'
        )
      end

      def render_person_selector_overflow
        return if overflow_selector_options.empty?

        render_person_selector_dropdown(
          options: overflow_selector_options,
          testid: 'dashboard-person-overflow',
          trigger_class: 'min-w-44',
          content_class: 'w-56'
        )
      end

      def render_person_selector_dropdown(options:, testid:, trigger_class:, content_class:)
        render RubyUI::DropdownMenu.new(
          class: trigger_class,
          data: { testid: testid }
        ) do
          render RubyUI::DropdownMenuTrigger.new(class: 'w-full') do
            button(
              type: 'button',
              class: 'inline-flex min-h-12 w-full items-center justify-between gap-2 rounded-xl border ' \
                     'border-outline-variant bg-surface-container-low px-3 py-2 text-sm font-bold ' \
                     'text-on-surface-variant shadow-sm hover:bg-surface-container-high ' \
                     'focus:outline-none focus:ring-2 focus:ring-primary/20',
              aria: { label: t('dashboard.person_selector.more_people') }
            ) do
              span(class: 'truncate') { overflow_selector_label }
              render Icons::ChevronsUpDown.new(size: 16, class: 'shrink-0 opacity-70')
            end
          end
          render RubyUI::DropdownMenuContent.new(class: content_class) do
            options.each do |option|
              render RubyUI::DropdownMenuItem.new(
                href: dashboard_path(dashboard_person_id: option.fetch(:id)),
                class: overflow_selector_item_class(option),
                aria: option.fetch(:selected) ? { current: 'true' } : {}
              ) do
                option.fetch(:label)
              end
            end
          end
        end
      end

      def selected_person_selector_option
        dashboard_person_options.find { |option| option.fetch(:selected) }
      end

      def direct_person_options
        person_selector_options.first(DIRECT_PERSON_OPTION_LIMIT)
      end

      def overflow_selector_options
        person_selector_options.drop(DIRECT_PERSON_OPTION_LIMIT) + all_family_options
      end

      def mobile_overflow_selector_options
        dashboard_person_options.reject { |option| option.fetch(:selected) }
      end

      def overflow_selector_label
        selected_option = overflow_selector_options.find { |option| option.fetch(:selected) }
        return selected_option.fetch(:label) if selected_option

        t('dashboard.person_selector.more_people')
      end

      def overflow_selector_item_class(option)
        return 'bg-primary text-on-primary hover:bg-primary hover:text-on-primary' if option.fetch(:selected)

        nil
      end

      def person_selector_options
        dashboard_person_options.reject { |option| option.fetch(:all_family) }
      end

      def all_family_options
        dashboard_person_options.select { |option| option.fetch(:all_family) }
      end

      def render_stats_section
        div(class: 'grid grid-cols-1 sm:grid-cols-3 auto-rows-fr gap-3 mb-8') do
          render Components::Shared::MetricCard.new(
            title: t('dashboard.stats.next_due'),
            value: next_due_value,
            icon_type: 'clock',
            layout: :compact
          )
          render Components::Shared::MetricCard.new(
            title: t('dashboard.stats.due_now'),
            value: due_now_count,
            icon_type: 'active_schedules',
            layout: :compact
          )
          render Components::Shared::MetricCard.new(
            title: t('dashboard.stats.tasks_left'),
            value: tasks_left_count,
            icon_type: 'check',
            layout: :compact
          )
        end
      end

      def render_timeline_section
        div(class: 'space-y-6') do
          div(class: 'flex items-center justify-between mb-2 px-1') do
            m3_heading(variant: :title_large, level: 2, class: 'font-bold tracking-tight') do
              t('dashboard.todays_schedule')
            end
          end

          task_people = people_with_dashboard_items
          if task_people.any?
            div(class: 'space-y-4') do
              task_people.each do |person|
                render Components::Dashboard::PersonTaskCard.new(
                  person: person,
                  routine_tasks: routine_tasks_by_person.fetch(person, []),
                  as_needed_items: as_needed_by_person.fetch(person, []),
                  current_user: current_user
                )
              end
            end
          else
            m3_card(variant: :filled,
                    class: 'p-16 text-center rounded-[2.5rem] border-dashed border-2 ' \
                           'border-outline-variant/50 bg-surface-container-low') do
              m3_text(variant: :body_large, class: 'text-on-surface-variant font-medium italic') do
                t('dashboard.empty_state')
              end
            end
          end
        end
      end

      def people_with_dashboard_items
        people.select do |person|
          routine_tasks_by_person.fetch(person, []).any? || as_needed_by_person.fetch(person, []).any?
        end
      end

      def render_health_insights
        div(class: 'space-y-4 pt-4') do
          m3_heading(variant: :title_large, level: 2, class: 'font-bold tracking-tight') do
            t('dashboard.insights.title')
          end
          m3_card(
            variant: :filled,
            class: 'rounded-[2.5rem] p-10 relative overflow-hidden border-none transition-all duration-300 ' \
                   "#{dashboard_insight_card_classes}"
          ) do
            div(class: 'relative z-10') do
              render_dashboard_insight_content
            end
          end
        end
      end

      def render_today_dose_history
        history = today_dose_history_by_person
        return if history.empty?

        section(class: 'space-y-4 pt-4', data: { testid: 'dashboard-today-dose-history' }) do
          m3_heading(variant: :title_large, level: 2, class: 'font-bold tracking-tight') do
            t('dashboard.dose_history.title')
          end
          div(class: 'space-y-3') do
            history.each do |person, takes|
              render_person_dose_history(person, takes)
            end
          end
        end
      end

      def render_person_dose_history(person, takes)
        m3_card(
          variant: :elevated,
          class: 'rounded-[2rem] border-none bg-surface-container-low p-5 shadow-elevation-1'
        ) do
          div(class: 'mb-4 flex items-center gap-3') do
            render Components::Shared::PersonAvatar.new(person: person, size: :sm)
            m3_heading(variant: :title_medium, level: 3, class: 'font-black tracking-tight') { person.name }
          end
          div(class: 'space-y-2') do
            takes.each { |take| render_dose_history_row(take) }
          end
        end
      end

      def render_dose_history_row(take)
        div(class: 'flex items-center justify-between gap-3 rounded-shape-xl bg-surface-container px-4 py-3') do
          div(class: 'min-w-0') do
            m3_text(variant: :body_medium, class: 'truncate font-bold text-foreground') do
              take.medication.display_name
            end
            m3_text(variant: :body_small, class: 'text-on-surface-variant font-medium') do
              DoseAmount.new(take.dose_amount, take.dose_unit).to_s
            end
          end
          span(class: 'shrink-0 text-sm font-black tabular-nums text-primary') do
            take.taken_at.strftime('%H:%M')
          end
        end
      end

      def today_dose_history_by_person
        people.each_with_object({}) do |person, history|
          takes = today_takes_for_person(person)
          history[person] = takes if takes.any?
        end
      end

      def today_takes_for_person(person)
        today_takes_by_person.fetch(person, [])
      end

      def render_dashboard_insight_content
        insight = presenter.smart_insights.primary_insight

        if presenter.smart_insights.learning_state?
          render_dashboard_insight_state(
            icon: Icons::Activity,
            content: {
              title: t('smart_insights.learning.title'),
              summary: t('smart_insights.learning.summary'),
              detail: t('smart_insights.learning.detail')
            }
          )
        elsif insight
          render_dashboard_primary_insight(insight)
        else
          render_dashboard_insight_state(
            icon: Icons::CheckCircle,
            content: {
              title: t('smart_insights.no_action.title'),
              summary: t('smart_insights.no_action.summary'),
              detail: t('smart_insights.no_action.detail')
            }
          )
        end
      end

      def render_dashboard_primary_insight(insight)
        render_dashboard_insight_state(
          icon: dashboard_insight_icon(insight),
          content: {
            title: insight.title,
            summary: insight.summary,
            detail: insight.detail,
            metric_label: insight.metric_label,
            metric_value: insight.metric_value
          }
        )
      end

      def render_dashboard_insight_state(icon:, content:)
        div(class: dashboard_insight_icon_tile_classes) do
          render icon.new(size: 24)
        end
        m3_heading(variant: :headline_small, level: 3, class: 'font-black mb-2 tracking-tight') do
          content.fetch(:title)
        end
        m3_text(variant: :body_large, class: dashboard_insight_body_text_classes) { content.fetch(:summary) }
        m3_text(variant: :body_medium, class: dashboard_insight_detail_text_classes) { content.fetch(:detail) }
        render_dashboard_insight_actions(content)
      end

      def render_dashboard_insight_actions(content)
        return if content[:metric_label].blank? && !presenter.can_view_reports?

        div(class: 'flex flex-wrap items-center gap-3 pt-6', data: { testid: 'dashboard-insight-actions' }) do
          render_dashboard_insight_metric(content)
          render_dashboard_insight_link
        end
      end

      def render_dashboard_insight_metric(content)
        return if content[:metric_label].blank? || content[:metric_value].blank?

        div(class: dashboard_insight_metric_tile_classes, data: { testid: 'dashboard-insight-metric' }) do
          span(class: 'text-xs font-bold uppercase tracking-widest') { content.fetch(:metric_label) }
          span(class: 'text-sm font-black') { content.fetch(:metric_value) }
        end
      end

      def render_dashboard_insight_link
        return unless presenter.can_view_reports?

        m3_link(
          href: reports_path(anchor: 'insights'),
          variant: :tonal,
          class: 'inline-flex max-w-full items-center justify-center rounded-shape-full px-4 py-2 ' \
                 'min-h-[44px] h-auto text-center text-sm font-bold uppercase tracking-wide leading-snug ' \
                 "#{dashboard_insight_link_classes} shadow-elevation-1 transition-all"
        ) do
          t('dashboard.insights.view_report')
        end
      end

      def dashboard_insight_card_classes
        insight = presenter.smart_insights.primary_insight
        if insight.blank?
          return 'bg-surface-container-low text-on-surface border border-outline-variant/70 shadow-elevation-1'
        end

        {
          urgent: 'bg-error-container text-on-error-container shadow-elevation-2',
          warning: 'bg-tertiary-container text-on-tertiary-container shadow-elevation-2',
          positive: 'bg-primary text-on-primary shadow-elevation-2',
          info: 'bg-secondary-container text-on-secondary-container shadow-elevation-1'
        }.fetch(insight.severity, 'bg-secondary-container text-on-secondary-container shadow-elevation-1')
      end

      def dashboard_insight_icon_classes
        presenter.smart_insights.primary_insight.present? ? 'bg-white/20' : 'bg-primary/10 text-primary'
      end

      def dashboard_insight_icon_tile_classes
        "w-12 h-12 rounded-2xl #{dashboard_insight_icon_classes} " \
          'flex items-center justify-center mb-6 shadow-inner'
      end

      def dashboard_insight_body_classes
        presenter.smart_insights.primary_insight.present? ? 'text-current/90' : 'text-on-surface'
      end

      def dashboard_insight_body_text_classes
        "#{dashboard_insight_body_classes} leading-relaxed font-medium"
      end

      def dashboard_insight_detail_classes
        presenter.smart_insights.primary_insight.present? ? 'text-current/80' : 'text-on-surface-variant'
      end

      def dashboard_insight_detail_text_classes
        "#{dashboard_insight_detail_classes} leading-relaxed mt-3"
      end

      def dashboard_insight_metric_classes
        presenter.smart_insights.primary_insight.present? ? 'bg-white/20' : 'bg-primary/10 text-primary'
      end

      def dashboard_insight_metric_tile_classes
        'inline-flex max-w-full items-center gap-2 rounded-shape-full px-4 py-2 min-h-[44px] ' \
          "#{dashboard_insight_metric_classes}"
      end

      def dashboard_insight_link_classes
        if presenter.smart_insights.primary_insight.present?
          'bg-surface-container-low text-on-surface border border-outline-variant/70 hover:bg-surface-container-high'
        else
          'bg-primary text-on-primary hover:bg-primary/90'
        end
      end

      def dashboard_insight_icon(insight)
        return Icons::AlertCircle if %i[urgent warning].include?(insight.severity)

        Icons::CheckCircle
      end

      def render_supply_levels
        div(class: 'space-y-6') do
          m3_heading(variant: :title_large, level: 2, class: 'font-bold tracking-tight') do
            t('dashboard.inventory.title')
          end
          m3_card(
            variant: :elevated,
            class: 'bg-surface-container-low p-8 rounded-[2.5rem] border-none shadow-elevation-1 transition-all ' \
                   'duration-300 hover:shadow-elevation-2 cursor-default'
          ) do
            div(class: 'space-y-8') do
              active_schedules.take(3).each do |p|
                render_supply_item(p.medication)
              end
              m3_link(
                href: medications_path,
                variant: :tonal,
                size: :lg,
                class: 'w-full font-black uppercase tracking-widest transition-all'
              ) do
                t('dashboard.inventory.order_refills')
              end
            end
          end
        end
      end

      def render_supply_item(medication)
        current = ::Medications::SupplyStatusPresenter.new(medication: medication).inventory_units_label
        percentage = medication.supply_percentage

        div(class: 'space-y-3') do
          div(class: 'flex justify-between items-center') do
            m3_text(variant: :label_large, class: 'font-black text-foreground uppercase tracking-tight') do
              medication.display_name
            end
            m3_text(variant: :label_medium, class: 'text-on-surface-variant font-black') do
              t('dashboard.inventory.left', count: current)
            end
          end
          div(class: 'h-3 w-full bg-surface-container rounded-full overflow-hidden shadow-inner') do
            progress(
              class: "supply-progress #{medication.low_stock? ? 'text-error' : 'text-primary'}",
              value: percentage,
              max: 100,
              aria: { label: t('dashboard.inventory.left', count: current) }
            )
          end
        end
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

      def render_version_footer
        div(class: 'mt-16 pt-6 border-t border-outline-variant/30 text-center') do
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

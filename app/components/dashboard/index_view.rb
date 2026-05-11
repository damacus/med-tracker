# frozen_string_literal: true

module Components
  module Dashboard
    # Dashboard index view component that renders the main dashboard page
    class IndexView < Components::Base
      attr_reader :presenter

      def initialize(presenter:)
        @presenter = presenter
        super()
      end

      def view_template
        div(class: 'container mx-auto max-w-6xl px-4 py-6', data: { testid: 'dashboard' }) do
          render_header
          render_stats_section

          div(class: 'grid grid-cols-1 lg:grid-cols-3 gap-8 items-start') do
            div(class: 'lg:col-span-2') do
              render_timeline_section
            end
            div(class: 'space-y-8') do
              render_supply_levels
            end
            div(class: 'lg:col-span-2') do
              render_health_insights
            end
          end
          render_version_footer
        end
      end

      private

      delegate :people, :active_schedules, :upcoming_schedules,
               :current_user, :doses, :next_dose_time, :routine_tasks_due?,
               :routine_tasks_by_person, :as_needed_by_person, :compliance_percentage, to: :presenter

      def next_dose_value
        time = next_dose_time
        return time.strftime('%H:%M') if time
        return t('dashboard.stats.due_today') if routine_tasks_due?

        t('dashboard.stats.no_upcoming_doses')
      end

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

      def render_stats_section
        div(class: 'grid grid-cols-2 lg:grid-cols-4 auto-rows-fr gap-3 mb-8') do
          render Components::Shared::MetricCard.new(
            title: t('dashboard.stats.people'),
            value: people.size,
            icon_type: 'users',
            href: people_path,
            layout: :compact
          )
          render Components::Shared::MetricCard.new(
            title: t('dashboard.stats.active_schedules'),
            value: active_schedules.size,
            icon_type: 'active_schedules',
            href: schedules_path,
            layout: :compact
          )
          render Components::Shared::MetricCard.new(
            title: t('dashboard.stats.compliance'),
            value: "#{compliance_percentage}%",
            icon_type: 'compliance',
            href: reports_path,
            layout: :compact
          )
          render Components::Shared::MetricCard.new(
            title: t('dashboard.stats.next_dose'),
            value: next_dose_value,
            icon_type: 'clock',
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
        render_dashboard_insight_metric(content)
        render_dashboard_insight_link
      end

      def render_dashboard_insight_metric(content)
        return if content[:metric_label].blank? || content[:metric_value].blank?

        div(class: dashboard_insight_metric_tile_classes) do
          span(class: 'text-xs font-bold uppercase tracking-widest') { content.fetch(:metric_label) }
          span(class: 'text-sm font-black') { content.fetch(:metric_value) }
        end
      end

      def render_dashboard_insight_link
        return unless presenter.can_view_reports?

        m3_link(
          href: reports_path(anchor: 'insights'),
          variant: :text,
          class: "mt-8 p-0 h-auto font-black uppercase tracking-widest #{dashboard_insight_link_classes} " \
                 'border-b-2 rounded-none hover:bg-transparent transition-all'
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
        "mt-6 inline-flex items-center gap-2 rounded-full px-3 py-1.5 #{dashboard_insight_metric_classes}"
      end

      def dashboard_insight_link_classes
        if presenter.smart_insights.primary_insight.present?
          'text-current border-current/30 hover:border-current'
        else
          'text-primary border-primary/30 hover:border-primary'
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
                class: 'w-full py-6 rounded-xl font-black uppercase tracking-widest transition-all'
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
            div(
              class: "h-full #{medication.low_stock? ? 'bg-error' : 'bg-primary'} " \
                     'rounded-full transition-all duration-1000',
              style: "width: #{percentage}%"
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

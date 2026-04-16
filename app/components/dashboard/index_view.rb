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
        div(class: 'container mx-auto px-4 py-8 pb-24 md:pb-8 max-w-6xl', data: { testid: 'dashboard' }) do
          render_header
          render_stats_section

          div(class: 'grid grid-cols-1 lg:grid-cols-3 gap-8 items-start') do
            div(class: 'lg:col-span-2') do
              render_timeline_section
            end
            div(class: 'space-y-8') do
              render_right_rail_next_dose
              render_schedules_section
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
               :current_user, :doses, :next_dose_time, :compliance_percentage, to: :presenter

      def next_dose_value
        time = next_dose_time
        time ? time.strftime('%H:%M') : t('dashboard.stats.no_upcoming_doses')
      end

      def render_header
        div(class: 'flex flex-col md:flex-row md:items-end justify-between gap-6 mb-12') do
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
          div(class: 'flex gap-3') do
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
        div(class: 'grid grid-cols-2 lg:grid-cols-4 auto-rows-fr gap-4 mb-12') do
          render Components::Shared::MetricCard.new(
            title: t('dashboard.stats.people'),
            value: people.size,
            icon_type: 'users',
            href: people_path
          )
          render Components::Shared::MetricCard.new(
            title: t('dashboard.stats.active_schedules'),
            value: active_schedules.size,
            icon_type: 'pill',
            href: schedules_path
          )
          render Components::Shared::MetricCard.new(
            title: t('dashboard.stats.compliance'),
            value: "#{compliance_percentage}%",
            icon_type: 'check',
            href: reports_path
          )
          render Components::Shared::MetricCard.new(
            title: t('dashboard.stats.next_dose'),
            value: next_dose_value,
            icon_type: 'clock'
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

          if doses.any?
            div(class: 'space-y-4') do
              doses.each do |dose|
                render Components::Dashboard::TimelineItem.new(dose: dose, current_user: current_user)
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

      def render_health_insights
        div(class: 'space-y-4 pt-4') do
          m3_heading(variant: :title_large, level: 2, class: 'font-bold tracking-tight') do
            t('dashboard.insights.title')
          end
          m3_card(
            variant: :filled,
            class: 'bg-primary rounded-[2.5rem] p-10 text-on-primary relative overflow-hidden border-none ' \
                   'transition-all duration-300 hover:shadow-xl hover:shadow-primary/30 ' \
                   'group cursor-default'
          ) do
            div(class: 'absolute -right-12 -top-12 w-48 h-48 bg-white/10 rounded-full blur-3xl ' \
                       'group-hover:bg-white/20 transition-all')
            div(class: 'relative z-10') do
              div(class: 'w-12 h-12 rounded-2xl bg-white/20 flex items-center justify-center mb-6 ' \
                         'group-hover:scale-110 transition-transform shadow-inner') do
                render Icons::AlertCircle.new(size: 24)
              end
              m3_heading(variant: :headline_small, level: 3, class: 'font-black mb-2 tracking-tight') do
                t('dashboard.insights.pattern_detected')
              end
              m3_text(variant: :body_large, class: 'text-on-primary/90 leading-relaxed mb-8 font-medium') do
                t('dashboard.insights.message')
              end
              m3_button(
                variant: :text,
                class: 'p-0 h-auto font-black uppercase tracking-widest text-on-primary ' \
                       'border-b-2 border-on-primary/30 ' \
                       'rounded-none hover:border-on-primary hover:bg-transparent transition-all'
              ) do
                t('dashboard.insights.view_report')
              end
            end
          end
        end
      end

      def render_schedules_section
        upcoming_doses = doses.reject { |d| d[:status] == :taken }.first(5)
        return if upcoming_doses.empty?

        div(class: 'space-y-6') do
          m3_heading(variant: :title_large, level: 2, class: 'font-bold tracking-tight') do
            t('dashboard.medication_schedule')
          end
          div(class: 'space-y-4') do
            upcoming_doses.each { |dose| upcoming_dose_item(dose) }
          end
        end
      end

      def render_right_rail_next_dose
        render Components::Shared::MetricCard.new(
          title: t('dashboard.stats.next_dose'),
          value: next_dose_value,
          icon_type: 'clock',
          layout: :compact,
          testid: 'dashboard-right-rail-next-dose'
        )
      end

      def upcoming_dose_item(dose)
        source = dose[:source]
        medication = source.medication
        time_str = dose[:scheduled_at]&.strftime('%H:%M') || '--:--'
        status = dose[:status]

        div(class: 'flex items-start gap-3 p-1') do
          div(class: "w-2 h-2 rounded-full mt-2 flex-shrink-0 #{status_dot_color(status)}")
          div do
            m3_text(weight: 'bold', size: '3') { "#{time_str} — #{medication.name}" }
            m3_text(size: '2', weight: 'muted') do
              "#{dose[:person].name} · #{dosage_label(source)}"
            end
          end
        end
      end

      def status_dot_color(status)
        case status
        when :upcoming then 'bg-primary'
        when :cooldown then 'bg-warning'
        when :out_of_stock then 'bg-destructive'
        else 'bg-primary/15'
        end
      end

      def dosage_label(source)
        if source.is_a?(::Schedule)
          "#{source.dose_amount} #{source.dose_unit}"
        else
          source.dose_display
        end
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
        current = medication.current_supply || 0
        percentage = medication.supply_percentage

        div(class: 'space-y-3') do
          div(class: 'flex justify-between items-center') do
            m3_text(variant: :label_large, class: 'font-black text-foreground uppercase tracking-tight') do
              medication.name
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

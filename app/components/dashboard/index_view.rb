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

          div(class: 'grid grid-cols-1 lg:grid-cols-3 gap-8') do
            div(class: 'lg:col-span-2 space-y-8') do
              render_timeline_section
              render_health_insights
            end
            div(class: 'space-y-8') do
              render_schedules_section
              render_supply_levels
            end
          end
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
            Text(size: '2', weight: 'muted', class: 'uppercase tracking-widest mb-1 block font-bold') do
              Time.current.strftime('%A, %b %d')
            end
            Heading(level: 1, size: '8', class: 'font-extrabold tracking-tight') do
              if current_user.person&.name.present?
                t(greeting_key, name: current_user.person.name.split.first)
              else
                t('dashboard.title')
              end
            end
          end
          div(class: 'flex gap-3') do
            if can_create_person?
              render RubyUI::Link.new(
                href: new_person_path,
                variant: :outline,
                size: :lg
              ) do
                t('dashboard.quick_actions.add_person')
              end
            end
            render RubyUI::Link.new(
              href: new_medication_path,
              variant: :primary,
              size: :lg
            ) do
              render Icons::PlusCircle.new(size: 20, class: 'mr-2')
              span { t('dashboard.quick_actions.add_medication') }
            end
          end
        end
      end

      def render_stats_section
        div(class: 'grid grid-cols-2 lg:grid-cols-4 gap-4 mb-12') do
          render Components::Dashboard::StatCard.new(
            title: t('dashboard.stats.people'),
            value: people.count,
            icon_type: 'users'
          )
          render Components::Dashboard::StatCard.new(
            title: t('dashboard.stats.active_schedules'),
            value: active_schedules.count,
            icon_type: 'pill'
          )
          render Components::Dashboard::StatCard.new(
            title: t('dashboard.stats.compliance'),
            value: "#{compliance_percentage}%",
            icon_type: 'check'
          )
          render Components::Dashboard::StatCard.new(
            title: t('dashboard.stats.next_dose'),
            value: next_dose_value,
            icon_type: 'clock'
          )
        end
      end

      def render_timeline_section
        div(class: 'space-y-6') do
          div(class: 'flex items-center justify-between mb-2') do
            Heading(level: 2, size: '5', class: 'font-bold') { t('dashboard.todays_schedule') }
          end

          if doses.any?
            div(class: 'space-y-4') do
              doses.each do |dose|
                render Components::Dashboard::TimelineItem.new(dose: dose, current_user: current_user)
              end
            end
          else
            render RubyUI::Card.new(class: 'p-12 text-center rounded-[2rem] border-dashed border-2') do
              Text(weight: :muted) { t('dashboard.empty_state') }
            end
          end
        end
      end

      def render_health_insights
        div(class: 'space-y-4 pt-4') do
          Heading(level: 2, size: '5', class: 'font-bold') { t('dashboard.insights.title') }
          render RubyUI::Card.new(
            class: 'bg-indigo-600 rounded-[2.5rem] p-8 text-white relative overflow-hidden border-none ' \
                   'transition-all duration-300 hover:scale-[1.02] hover:shadow-xl hover:shadow-indigo-500/20 ' \
                   'group cursor-default'
          ) do
            div(class: 'absolute -right-8 -top-8 w-32 h-32 bg-white/10 rounded-full blur-2xl ' \
                       'group-hover:bg-white/20 transition-all')
            div(class: 'relative z-10') do
              div(class: 'w-10 h-10 rounded-xl bg-white/20 flex items-center justify-center mb-6 ' \
                         'group-hover:scale-110 transition-transform') do
                render Icons::AlertCircle.new(size: 20)
              end
              Heading(level: 3, size: '4', class: 'font-bold mb-2') { t('dashboard.insights.pattern_detected') }
              Text(class: 'text-indigo-100 text-sm leading-relaxed mb-6') do
                t('dashboard.insights.message')
              end
              button(
                class: 'text-xs font-bold uppercase tracking-widest text-white border-b border-white/30 ' \
                       'pb-1 hover:border-white transition-all'
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
          Heading(level: 2, size: '5', class: 'font-bold') { t('dashboard.medication_schedule') }
          div(class: 'space-y-4') do
            upcoming_doses.each { |dose| upcoming_dose_item(dose) }
          end
        end
      end

      def upcoming_dose_item(dose)
        source = dose[:source]
        medication = source.medication
        time_str = dose[:scheduled_at]&.strftime('%H:%M') || '--:--'
        status = dose[:status]

        div(class: 'flex items-start gap-3 p-1') do
          div(class: "w-2 h-2 rounded-full mt-2 flex-shrink-0 #{status_dot_color(status)}")
          div do
            Text(weight: 'bold', size: '3') { "#{time_str} — #{medication.name}" }
            Text(size: '2', weight: 'muted') do
              "#{dose[:person].name} · #{dosage_label(source)}"
            end
          end
        end
      end

      def status_dot_color(status)
        case status
        when :upcoming then 'bg-primary'
        when :cooldown then 'bg-amber-400'
        when :out_of_stock then 'bg-destructive'
        else 'bg-slate-300'
        end
      end

      def dosage_label(source)
        if source.is_a?(Schedule)
          "#{source.dosage.amount} #{source.dosage.unit}"
        else
          source.medication.dosage_unit
        end
      end

      def render_supply_levels
        div(class: 'space-y-6') do
          Heading(level: 2, size: '5', class: 'font-bold') { t('dashboard.inventory.title') }
          render RubyUI::Card.new(
            class: 'bg-white p-8 rounded-[2.5rem] border border-slate-100 shadow-sm transition-all ' \
                   'duration-300 hover:shadow-md hover:scale-[1.01] cursor-default'
          ) do
            div(class: 'space-y-6') do
              active_schedules.take(3).each do |p|
                render_supply_item(p.medication)
              end
              render RubyUI::Link.new(
                href: medications_path,
                variant: :ghost,
                class: 'w-full py-4 rounded-2xl bg-slate-50 text-slate-500 text-xs font-bold ' \
                       'hover:bg-slate-100 transition-all uppercase tracking-widest no-underline flex justify-center'
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

        div(class: 'space-y-2') do
          div(class: 'flex justify-between items-center text-xs') do
            span(class: 'font-bold') { medication.name }
            span(class: 'text-slate-400 font-bold') { t('dashboard.inventory.left', count: current) }
          end
          div(class: 'h-2 w-full bg-slate-100 rounded-full overflow-hidden') do
            div(
              class: "h-full #{medication.low_stock? ? 'bg-destructive' : 'bg-primary'} " \
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
    end
  end
end

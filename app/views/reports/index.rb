# frozen_string_literal: true

module Views
  module Reports
    class Index < Views::Base
      def initialize(daily_data:, inventory_alerts:, start_date:, end_date:)
        @daily_data = daily_data
        @inventory_alerts = inventory_alerts
        @start_date = start_date
        @end_date = end_date
        super()
      end

      def view_template
        div(class: 'container mx-auto px-4 py-12 max-w-5xl space-y-12') do
          render_header
          render_summary_card
          render_compliance_section
          render_insights_grid
        end
      end

      private

      # rubocop:disable Metrics/AbcSize
      def render_header
        div(class: 'flex flex-col md:flex-row items-center justify-between gap-4') do
          div(class: 'text-center md:text-left space-y-2') do
            Text(size: '2', weight: 'muted', class: 'uppercase tracking-[0.2em] font-black opacity-40') { 'Wellness Analytics' }
            Heading(level: 1, size: '9', class: 'font-black tracking-tight text-foreground') { 'Health Report' }
            p(class: 'text-slate-400') { "#{@start_date.strftime('%B %d')} — #{@end_date.strftime('%B %d, %Y')}" }
          end

          form(action: helpers.reports_path, method: :get, class: 'flex items-center gap-2 bg-white/50 backdrop-blur-sm p-4 rounded-xl border border-slate-100 shadow-sm') do
            div(class: 'flex flex-col gap-1') do
              label(for: 'start_date', class: 'text-xs font-bold text-slate-500 uppercase tracking-wider') { 'From' }
              input(type: 'date', name: 'start_date', id: 'start_date', value: @start_date, class: 'form-input text-sm rounded-lg border-slate-200 focus:border-indigo-500 focus:ring-indigo-500')
            end
            div(class: 'flex flex-col gap-1') do
              label(for: 'end_date', class: 'text-xs font-bold text-slate-500 uppercase tracking-wider') { 'To' }
              input(type: 'date', name: 'end_date', id: 'end_date', value: @end_date, class: 'form-input text-sm rounded-lg border-slate-200 focus:border-indigo-500 focus:ring-indigo-500')
            end
            button(type: 'submit', class: 'mt-5 bg-indigo-600 hover:bg-indigo-700 text-white p-2 rounded-lg transition-colors aria-label: "Apply date filters"') do
              render Icons::ChevronRight.new(size: 20)
            end
          end
        end
      end
      # rubocop:enable Metrics/AbcSize

      # rubocop:disable Metrics/AbcSize
      def render_summary_card
        total_expected = @daily_data.sum { |d| d[:expected] }
        total_actual = @daily_data.sum { |d| d[:actual] }
        overall_compliance = total_expected.zero? ? 100 : [(total_actual.to_f / total_expected * 100).round, 100].min

        Card(class: 'overflow-hidden border-none shadow-2xl') do
          div(class: 'bg-gradient-to-br from-indigo-600 to-violet-700 p-8 sm:p-12 text-white relative') do
            # Decorative element
            div(class: 'absolute right-0 top-0 w-64 h-64 bg-white/10 rounded-full blur-3xl -translate-y-1/2 translate-x-1/2')

            div(class: 'relative z-10 grid grid-cols-1 md:grid-cols-3 gap-8 text-center') do
              summary_stat('Overall Compliance', "#{overall_compliance}%", overall_compliance >= 90 ? 'Excellent' : 'Needs attention')
              summary_stat('Total Doses Logged', "#{total_actual}/#{total_expected}", total_actual >= total_expected ? 'On Track' : 'Missed doses')
              summary_stat('Current Health Status', 'Optimal', 'Vibrant') # This could be dynamic in the future based on compliance
            end
          end
        end
      end
      # rubocop:enable Metrics/AbcSize

      def summary_stat(label, value, subtext)
        div(class: 'space-y-1') do
          Text(size: '1', class: 'uppercase tracking-widest text-indigo-100 font-bold opacity-70') { label }
          Heading(level: 2, size: '8', class: 'font-black') { value }
          Text(size: '1', class: 'font-bold bg-white/20 inline-block px-2 py-0.5 rounded-full') { subtext }
        end
      end

      def render_compliance_section
        div(class: 'space-y-8') do
          div(class: 'flex items-center justify-between px-2') do
            Heading(level: 2, size: '5', class: 'font-bold') { 'Adherence Timeline' }
            render Button.new(variant: :outline, size: :sm, class: 'rounded-full') { 'Download PDF' }
          end

          Card(class: 'p-8 sm:p-10 border-slate-50 bg-white/50 backdrop-blur-sm') do
            div(class: 'flex items-end justify-between h-64 gap-4 px-2') do
              @daily_data.each do |day|
                render_bar(day)
              end
            end
          end
        end
      end

      def render_bar(day)
        div(class: 'flex-1 flex flex-col items-center gap-4 h-full justify-end group') do
          # The Bar
          div(class: 'relative w-full h-full flex flex-col justify-end') do
            # Background guide
            div(class: 'absolute inset-0 bg-slate-50 rounded-t-xl opacity-50')

            # Active Bar
            div(
              class: "relative w-full rounded-t-xl transition-all duration-700 group-hover:scale-x-105 #{day[:percentage] < 90 ? 'bg-indigo-300' : 'bg-primary'}",
              style: "height: #{day[:percentage]}%"
            ) do
              # Tooltip-like value on hover
              div(class: 'absolute -top-10 left-1/2 -translate-x-1/2 bg-slate-900 text-white text-[10px] font-bold px-2 py-1 rounded opacity-0 group-hover:opacity-100 transition-opacity whitespace-nowrap') do
                "#{day[:percentage]}% Compliance"
              end
            end
          end

          # Day Label
          Text(size: '1', weight: 'black', class: 'text-slate-400 uppercase tracking-tighter') { day[:day_name] }
        end
      end

      def render_insights_grid
        div(class: 'grid grid-cols-1 md:grid-cols-2 gap-8') do
          render_achievement_streak
          render_inventory_alert
        end
      end

      def render_achievement_streak
        render_insight_card(
          title: 'Achievement Streak',
          value: '4 Days Uninterrupted',
          description: "You haven't missed a single dose since Friday morning. Your body is maintaining optimal levels.",
          icon_class: Icons::CheckCircle,
          text_color: 'text-emerald-600',
          bg_color: 'bg-emerald-50'
        )
      end

      def render_inventory_alert
        alert = @inventory_alerts.first
        return unless alert

        medication_name = alert[:medication_name]
        days_left = alert[:days_left]

        description = if days_left <= 0
                        "Your #{medication_name} supply is exhausted. Please order an urgent refill."
                      else
                        "Your #{medication_name} supply will be exhausted in #{pluralize(days_left, 'day')}. We recommend ordering a refill this afternoon."
                      end

        render_insight_card(
          title: 'Inventory Alert',
          value: days_left <= 0 ? 'Out of Stock' : 'Refill Pending',
          description: description,
          icon_class: Icons::AlertCircle,
          text_color: 'text-rose-600',
          bg_color: 'bg-rose-50'
        )
      end

      # rubocop:disable Metrics/ParameterLists
      def render_insight_card(title:, value:, description:, icon_class:, text_color:, bg_color:)
        Card(class: "border-none shadow-sm #{bg_color} p-8 space-y-4 transition-transform hover:scale-[1.02]") do
          div(class: 'flex items-center gap-4') do
            div(class: "w-12 h-12 rounded-2xl flex items-center justify-center #{text_color} bg-white shadow-sm") do
              render icon_class.new(size: 24)
            end
            div do
              Heading(level: 3, size: '4', class: "#{text_color} font-black") { title }
              Text(size: '1', weight: 'bold', class: 'uppercase tracking-widest opacity-50') { 'Actionable insight' }
            end
          end

          div(class: 'space-y-2') do
            Heading(level: 4, size: '5', class: 'font-bold') { value }
            Text(size: '2', class: 'text-slate-600 leading-relaxed') { description }
          end
        end
      end
      # rubocop:enable Metrics/ParameterLists

      def pluralize(count, singular, plural = nil)
        "#{count} #{count == 1 ? singular : (plural || singular.pluralize)}"
      end
    end
  end
end

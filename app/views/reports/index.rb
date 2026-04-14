# frozen_string_literal: true

module Views
  module Reports
    InsightCard = Data.define(:title, :value, :description, :icon_class, :text_color, :bg_color)

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
            Text(size: '2', weight: 'muted', class: 'uppercase tracking-[0.2em] font-black opacity-40') do
              t('reports.index.eyebrow')
            end
            Heading(level: 1, size: '9', class: 'font-black tracking-tight text-foreground') { t('reports.index.title') }
            p(class: 'text-muted-foreground') { "#{@start_date.strftime('%B %d')} — #{@end_date.strftime('%B %d, %Y')}" }
          end

          form(action: view_context.reports_path, method: :get, class: 'flex items-center gap-2 rounded-xl border border-border bg-card/70 p-4 shadow-sm backdrop-blur-sm') do
            div(class: 'flex flex-col gap-1') do
              label(for: 'start_date', class: 'text-xs font-bold uppercase tracking-wider text-muted-foreground') { t('reports.index.start_date_label') }
              input(type: 'date', name: 'start_date', id: 'start_date', value: @start_date, class: 'form-input rounded-lg border-border bg-background text-sm text-foreground focus:border-primary focus:ring-primary')
            end
            div(class: 'flex flex-col gap-1') do
              label(for: 'end_date', class: 'text-xs font-bold uppercase tracking-wider text-muted-foreground') { t('reports.index.end_date_label') }
              input(type: 'date', name: 'end_date', id: 'end_date', value: @end_date, class: 'form-input rounded-lg border-border bg-background text-sm text-foreground focus:border-primary focus:ring-primary')
            end
            render Button.new(type: 'submit', class: 'mt-5', 'aria-label': t('reports.index.apply_filters_aria_label')) do
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
            div(class: 'absolute right-0 top-0 h-64 w-64 rounded-full bg-background/10 blur-3xl -translate-y-1/2 translate-x-1/2')

            div(class: 'relative z-10 grid grid-cols-1 md:grid-cols-3 gap-8 text-center') do
              summary_stat(
                t('reports.index.summary.overall_compliance'),
                "#{overall_compliance}%",
                overall_compliance >= 90 ? t('reports.index.summary.excellent') : t('reports.index.summary.needs_attention')
              )
              summary_stat(
                t('reports.index.summary.total_doses_logged'),
                "#{total_actual}/#{total_expected}",
                total_actual >= total_expected ? t('reports.index.summary.on_track') : t('reports.index.summary.missed_doses')
              )
              summary_stat(
                t('reports.index.summary.current_health_status'),
                t('reports.index.summary.optimal'),
                t('reports.index.summary.vibrant')
              ) # This could be dynamic in the future based on compliance
            end
          end
        end
      end
      # rubocop:enable Metrics/AbcSize

      def summary_stat(label, value, subtext)
        div(class: 'space-y-1') do
          Text(size: '1', class: 'uppercase tracking-widest text-indigo-100 font-bold opacity-70') { label }
          Heading(level: 2, size: '8', class: 'font-black') { value }
          Text(size: '1', class: 'inline-block rounded-full bg-background/15 px-2 py-0.5 font-bold') { subtext }
        end
      end

      def render_compliance_section
        div(class: 'space-y-8') do
          div(class: 'flex items-center justify-between px-2') do
            Heading(level: 2, size: '5', class: 'font-bold') { t('reports.index.timeline_title') }
            render Button.new(variant: :outline, size: :sm, class: 'rounded-full') { t('reports.index.download_pdf') }
          end

          Card(class: 'border-border bg-card/70 p-8 backdrop-blur-sm sm:p-10') do
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
            div(class: 'absolute inset-0 rounded-t-xl bg-muted/60')

            # Active Bar
            div(
              class: "relative w-full rounded-t-xl transition-all duration-700 group-hover:scale-x-105 #{day[:percentage] < 90 ? 'bg-indigo-300' : 'bg-primary'}",
              style: "height: #{day[:percentage]}%"
            ) do
              # Tooltip-like value on hover
              div(class: 'absolute -top-10 left-1/2 -translate-x-1/2 rounded bg-foreground px-2 py-1 text-[10px] font-bold whitespace-nowrap text-background opacity-0 transition-opacity group-hover:opacity-100') do
                t('reports.index.timeline_compliance', percentage: day[:percentage])
              end
            end
          end

          # Day Label
          Text(size: '1', weight: 'black', class: 'text-muted-foreground uppercase tracking-tighter') { day[:day_name] }
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
          InsightCard.new(
            title: t('reports.index.achievement_streak.title'),
            value: t('reports.index.achievement_streak.value'),
            description: t('reports.index.achievement_streak.description'),
            icon_class: Icons::CheckCircle,
            text_color: 'text-emerald-600',
            bg_color: 'bg-success-light'
          )
        )
      end

      def render_inventory_alert
        alert = @inventory_alerts.first
        return unless alert

        medication_name = alert[:medication_name]
        days_left = alert[:days_left]

        description = if days_left <= 0
                        t('reports.index.inventory_alert.description_zero', medication_name:)
                      else
                        t('reports.index.inventory_alert.description', medication_name:, count: days_left)
                      end

        render_insight_card(
          InsightCard.new(
            title: t('reports.index.inventory_alert.title'),
            value: days_left <= 0 ? t('reports.index.inventory_alert.out_of_stock') : t('reports.index.inventory_alert.refill_pending'),
            description: description,
            icon_class: Icons::AlertCircle,
            text_color: 'text-rose-600',
            bg_color: 'bg-destructive-light'
          )
        )
      end

      # rubocop:disable Metrics/AbcSize
      def render_insight_card(card)
        Card(class: "border-none shadow-sm #{card.bg_color} p-8 space-y-4 transition-transform hover:scale-[1.02]") do
          div(class: 'flex items-center gap-4') do
            div(class: "w-12 h-12 rounded-2xl flex items-center justify-center #{card.text_color} bg-card shadow-sm") do
              render card.icon_class.new(size: 24)
            end
            div do
              Heading(level: 3, size: '4', class: "#{card.text_color} font-black") { card.title }
              Text(size: '1', weight: 'bold', class: 'uppercase tracking-widest opacity-50') { t('reports.index.actionable_insight') }
            end
          end

          div(class: 'space-y-2') do
            Heading(level: 4, size: '5', class: 'font-bold') { card.value }
            Text(size: '2', class: 'text-muted-foreground leading-relaxed') { card.description }
          end
        end
      end

      # rubocop:enable Metrics/AbcSize
    end
  end
end

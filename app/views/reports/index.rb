# frozen_string_literal: true

module Views
  module Reports
    InsightCard = Data.define(:title, :value, :description, :icon_class, :text_color, :icon_background_class)

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
        div(class: 'mb-12 flex flex-col gap-6 md:flex-row md:items-end md:justify-between') do
          div(class: 'space-y-2 text-center md:text-left') do
            m3_text(size: '2', weight: 'muted', class: 'font-bold uppercase tracking-widest text-on-surface-variant') do
              t('reports.index.eyebrow')
            end
            m3_heading(level: 1, size: '8', class: 'font-extrabold tracking-tight text-foreground') { t('reports.index.title') }
            p(class: 'text-on-surface-variant') { "#{@start_date.strftime('%B %d')} — #{@end_date.strftime('%B %d, %Y')}" }
          end

          form(action: view_context.reports_path, method: :get, class: 'flex items-end gap-3 rounded-[1.5rem] border border-border/70 bg-popover p-4 shadow-elevation-1') do
            div(class: 'flex flex-col gap-1') do
              label(for: 'start_date', class: 'text-xs font-semibold uppercase tracking-wider text-on-surface-variant') { t('reports.index.start_date_label') }
              input(type: 'date', name: 'start_date', id: 'start_date', value: @start_date, class: 'form-input rounded-lg border-border bg-background text-sm text-foreground focus:border-primary focus:ring-primary')
            end
            div(class: 'flex flex-col gap-1') do
              label(for: 'end_date', class: 'text-xs font-semibold uppercase tracking-wider text-on-surface-variant') { t('reports.index.end_date_label') }
              input(type: 'date', name: 'end_date', id: 'end_date', value: @end_date, class: 'form-input rounded-lg border-border bg-background text-sm text-foreground focus:border-primary focus:ring-primary')
            end
            m3_button(type: 'submit', class: 'rounded-xl shadow-elevation-1', 'aria-label': t('reports.index.apply_filters_aria_label')) do
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

        m3_card(class: 'overflow-hidden border border-border/70 bg-primary text-primary-foreground shadow-elevation-3') do
          div(class: 'relative p-8 sm:p-12') do
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
          m3_text(size: '1', class: 'font-bold uppercase tracking-widest text-primary-foreground/85') { label }
          m3_heading(level: 2, size: '8', class: 'font-black') { value }
          m3_text(size: '1', class: 'inline-block rounded-full bg-primary-foreground/20 px-2 py-0.5 font-bold text-primary-foreground') { subtext }
        end
      end

      def render_compliance_section
        div(class: 'space-y-8') do
          div(class: 'flex items-center justify-between px-2') do
            m3_heading(level: 2, size: '5', class: 'font-bold') { t('reports.index.timeline_title') }
            m3_button(variant: :outlined, size: :sm, class: 'rounded-full') { t('reports.index.download_pdf') }
          end

          m3_card(class: 'border border-border/70 bg-card p-8 shadow-elevation-2 sm:p-10') do
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
            div(class: 'absolute inset-0 rounded-t-xl bg-border/40')

            # Active Bar
            div(
              class: "relative w-full rounded-t-xl transition-all duration-700 group-hover:scale-x-105 #{day[:percentage] < 90 ? 'bg-primary/40' : 'bg-primary'}",
              style: "height: #{day[:percentage]}%"
            ) do
              # Tooltip-like value on hover
              div(class: 'absolute -top-10 left-1/2 -translate-x-1/2 rounded bg-foreground px-2 py-1 text-[10px] font-bold whitespace-nowrap text-background opacity-0 transition-opacity group-hover:opacity-100') do
                t('reports.index.timeline_compliance', percentage: day[:percentage])
              end
            end
          end

          # Day Label
          m3_text(size: '1', weight: 'bold', class: 'text-on-surface-variant uppercase tracking-tighter font-black') { day[:day_name] }
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
            icon_background_class: 'bg-emerald-50'
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
            icon_background_class: 'bg-rose-50'
          )
        )
      end

      # rubocop:disable Metrics/AbcSize
      def render_insight_card(card)
        m3_card(class: 'space-y-4 border border-border/70 bg-card p-8 shadow-elevation-1 transition-transform hover:scale-[1.02] hover:shadow-elevation-2') do
          div(class: 'flex items-center gap-4') do
            div(class: "flex h-12 w-12 items-center justify-center rounded-2xl #{card.icon_background_class} #{card.text_color}") do
              render card.icon_class.new(size: 24)
            end
            div do
              m3_heading(level: 3, size: '4', class: "#{card.text_color} font-black") { card.title }
              m3_text(size: '1', weight: 'bold', class: 'uppercase tracking-widest opacity-50') { t('reports.index.actionable_insight') }
            end
          end

          div(class: 'space-y-2') do
            m3_heading(level: 4, size: '5', class: 'font-bold') { card.value }
            m3_text(size: '2', class: 'text-on-surface-variant leading-relaxed') { card.description }
          end
        end
      end

      # rubocop:enable Metrics/AbcSize
    end
  end
end

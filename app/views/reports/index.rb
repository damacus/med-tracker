# frozen_string_literal: true

module Views
  module Reports
    InsightCard = Data.define(:title, :value, :description, :icon_class, :text_color, :icon_background_class)

    class Index < Views::Base
      def initialize(daily_data:, smart_insights:, start_date:, end_date:, **options)
        @daily_data = daily_data
        @smart_insights = smart_insights
        @start_date = start_date
        @end_date = end_date
        @today_taken_medications = options.fetch(:today_taken_medications)
        @people = options.fetch(:people)
        @selected_person_id = options.fetch(:selected_person_id)
        super()
      end

      def view_template
        div(class: 'container mx-auto px-4 py-12 max-w-5xl space-y-12') do
          render_header
          render_today_section
          render_summary_card
          render_compliance_section
          render_insights_grid
        end
      end

      private

      def render_header
        div(class: 'mb-12 flex flex-col gap-6 md:flex-row md:items-end md:justify-between') do
          div(class: 'space-y-2 text-center md:text-left') do
            m3_text(size: '2', weight: 'muted', class: 'font-bold uppercase tracking-widest text-on-surface-variant') do
              t('reports.index.eyebrow')
            end
            m3_heading(level: 1, size: '8', class: 'font-extrabold tracking-tight text-foreground') { t('reports.index.title') }
            p(class: 'text-on-surface-variant') { "#{@start_date.strftime('%B %d')} — #{@end_date.strftime('%B %d, %Y')}" }
          end

          render Components::Reports::FilterForm.new(
            action_path: view_context.reports_path,
            people: @people,
            selected_person_id: @selected_person_id,
            start_date: @start_date,
            end_date: @end_date
          )
        end
      end

      def render_today_section = render Components::Reports::TodaySection.new(today_taken_medications: @today_taken_medications)

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
                t('reports.index.summary.evidence_reviewed'),
                @smart_insights.evidence_summary,
                t('reports.index.summary.based_on_logs')
              )
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
            m3_link(
              href: view_context.health_history_report_path(start_date: @start_date, end_date: @end_date,
                                                            person_id: @selected_person_id.presence),
              variant: :outlined, size: :sm
            ) { t('reports.index.download_pdf') }
          end

          m3_card(class: 'border border-border/70 bg-card p-8 shadow-elevation-2 sm:p-10') do
            div(class: 'flex items-end justify-between h-64 gap-4 px-2') do
              @daily_data.each { |day| render_bar(day) }
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
              class: "relative w-full rounded-t-xl transition-all duration-700 group-hover:scale-x-105 #{bar_height_class(day[:percentage])} #{day[:percentage] < 90 ? 'bg-primary/40' : 'bg-primary'}",
              data: { testid: 'report-compliance-bar' }
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

      def bar_height_class(percentage) = "report-compliance-bar-height-#{percentage.to_i.round(-1).clamp(0, 100)}"

      def render_insights_grid
        section(id: 'insights', class: 'space-y-6 scroll-mt-24') do
          render_insights_header

          if @smart_insights.learning_state?
            render_learning_state
          elsif @smart_insights.insights.any?
            render_smart_insight_cards
          else
            render_no_action_state
          end
        end
      end

      def render_insights_header
        div(class: 'space-y-2') do
          m3_heading(level: 2, size: '5', class: 'font-bold') { t('smart_insights.title') }
          m3_text(size: '2', class: 'text-on-surface-variant') { @smart_insights.evidence_summary }
        end
      end

      def render_learning_state
        render_state_card(
          title: t('smart_insights.learning.title'),
          summary: t('smart_insights.learning.summary'),
          detail: t('smart_insights.learning.detail')
        )
      end

      def render_no_action_state
        render_state_card(
          title: t('smart_insights.no_action.title'),
          summary: t('smart_insights.no_action.summary'),
          detail: t('smart_insights.no_action.detail')
        )
      end

      def render_smart_insight_cards
        div(class: 'grid grid-cols-1 md:grid-cols-2 gap-8') do
          @smart_insights.insights.each { |insight| render_smart_insight_card(insight) }
        end
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

      def render_smart_insight_card(insight)
        render_insight_card(
          InsightCard.new(
            title: insight.title,
            value: insight.summary,
            description: insight.detail,
            icon_class: insight_icon(insight),
            text_color: insight_text_color(insight),
            icon_background_class: insight_icon_background(insight)
          )
        )
      end

      def render_state_card(title:, summary:, detail:)
        m3_card(class: 'space-y-3 border border-border/70 bg-card p-8 shadow-elevation-1') do
          m3_heading(level: 3, size: '4', class: 'font-bold') { title }
          m3_text(size: '2', class: 'font-semibold text-on-surface') { summary }
          m3_text(size: '2', class: 'text-on-surface-variant leading-relaxed') { detail }
        end
      end

      def insight_icon(insight)
        return Icons::AlertCircle if %i[urgent warning].include?(insight.severity)

        Icons::CheckCircle
      end

      def insight_text_color(insight)
        {
          urgent: 'text-rose-600',
          warning: 'text-amber-700',
          positive: 'text-emerald-600',
          info: 'text-primary'
        }.fetch(insight.severity, 'text-primary')
      end

      def insight_icon_background(insight)
        {
          urgent: 'bg-rose-50',
          warning: 'bg-amber-50',
          positive: 'bg-emerald-50',
          info: 'bg-primary/10'
        }.fetch(insight.severity, 'bg-primary/10')
      end
    end
  end
end

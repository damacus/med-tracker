# frozen_string_literal: true

module Reports
  class HealthHistoryPdf
    attr_reader :result, :start_date, :end_date, :generated_at, :include_medication_takes

    def initialize(result:, start_date:, end_date:, generated_at:, include_medication_takes: false)
      @result = result
      @start_date = start_date
      @end_date = end_date
      @generated_at = generated_at
      @include_medication_takes = include_medication_takes
    end

    def render
      PdfRenderer.new.render(component: document, metadata:)
    end

    private

    def document
      Components::Reports::PdfDocument.new(
        title: t('title'),
        context: header_context,
        context_lines: header_context_lines,
        generated_at:,
        generated_at_text: t('generated_at', timestamp: generated_timestamp),
        content: report_component,
        locale: I18n.locale.to_s,
        page_number: gp_result? ? t('gp.page_number') : nil
      )
    end

    def metadata
      { title: t('title'), author: 'MedTracker' }
    end

    def header_context
      header_context_lines.join(' · ')
    end

    def header_context_lines
      return gp_header_context_lines if gp_result?

      [
        t('people', people: people_label),
        t('date_range', start_date: date(start_date), end_date: date(end_date))
      ]
    end

    def gp_header_context_lines
      [
        result.person.name,
        t('gp.reporting_period', start_date: date(start_date), end_date: date(end_date))
      ]
    end

    def people_label
      report_people.map(&:name).presence&.to_sentence || t('no_people')
    end

    def report_component
      return Components::Reports::GpHealthHistoryReport.new(result:, include_medication_takes:) if gp_result?

      Components::Reports::HealthHistoryReport.new(result:)
    end

    def gp_result?
      result.respond_to?(:person) && result.respond_to?(:current_medicines) && result.respond_to?(:chronology)
    end

    def report_people
      return [result.person] if gp_result?

      result.people
    end

    def generated_timestamp
      generated_at.in_time_zone.strftime('%Y-%m-%d %H:%M %Z')
    end

    def date(value)
      I18n.l(value)
    end

    def t(key, **)
      I18n.t("reports.health_history.#{key}", **)
    end
  end
end

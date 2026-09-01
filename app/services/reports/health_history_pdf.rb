# frozen_string_literal: true

module Reports
  class HealthHistoryPdf
    attr_reader :result, :start_date, :end_date, :generated_at

    def initialize(result:, start_date:, end_date:, generated_at:)
      @result = result
      @start_date = start_date
      @end_date = end_date
      @generated_at = generated_at
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
        content: Components::Reports::HealthHistoryReport.new(result:),
        locale: I18n.locale.to_s
      )
    end

    def metadata
      { title: t('title'), author: 'MedTracker' }
    end

    def header_context
      header_context_lines.join(' · ')
    end

    def header_context_lines
      [
        t('people', people: people_label),
        t('date_range', start_date: date(start_date), end_date: date(end_date))
      ]
    end

    def people_label
      result.people.map(&:name).presence&.to_sentence || t('no_people')
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

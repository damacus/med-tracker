# frozen_string_literal: true

module Reports
  class MedicationReviewPdf
    attr_reader :prompts, :generated_at

    def initialize(prompts:, generated_at:)
      @prompts = prompts
      @generated_at = generated_at
    end

    def render
      PdfRenderer.new.render(component: document, metadata:)
    end

    private

    def document
      Components::Reports::PdfDocument.new(
        title: t('title'),
        context: t('context'),
        generated_at:,
        generated_at_text: t('generated_at', timestamp: generated_timestamp),
        content: Components::Reports::MedicationReviewReport.new(prompts:),
        locale: I18n.locale.to_s
      )
    end

    def metadata
      { title: t('title'), author: 'MedTracker', subject: t('boundary') }
    end

    def generated_timestamp
      timestamp = generated_at.in_time_zone
      t('timestamp_format', date: date(timestamp.to_date), time: timestamp.strftime('%H:%M'), zone: timestamp.zone)
    end

    def date(value)
      t('date_format', day: value.day, month: t('months').fetch(value.month - 1), year: value.year)
    end

    def t(key, **)
      I18n.t("reports.medication_review.#{key}", **)
    end
  end
end

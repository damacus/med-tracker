# frozen_string_literal: true

module Components
  module MedicationReviews
    class PromptCard < Components::Base
      def initialize(prompt:)
        @prompt = prompt
        super()
      end

      def view_template
        article(class: 'overflow-hidden rounded-shape-lg border border-border bg-surface-container-lowest ' \
                       'shadow-elevation-1',
                data: { review_prompt_id: prompt.id }) do
          div(class: 'grid gap-6 p-5 md:grid-cols-[minmax(0,1.25fr)_minmax(18rem,0.75fr)] md:p-6') do
            render_evidence
            render_review_form
          end
        end
      end

      private

      attr_reader :prompt

      def render_evidence
        div(class: 'min-w-0') do
          div(class: 'mb-4 flex flex-wrap items-center gap-2') do
            m3_badge(variant: risk_badge_variant) { risk_label }
            m3_badge(variant: :outlined) { status_label }
            m3_badge(variant: :tonal) { t('medication_reviews.confidence', value: confidence_label) }
          end
          m3_heading(level: 3, size: '5', class: 'font-bold') do
            "#{prompt.primary_medication_name} + #{prompt.interacting_medication_name}"
          end
          m3_text(variant: :body_medium, class: 'mt-3 block leading-relaxed text-on-surface-variant') do
            t('medication_reviews.review_reason')
          end
          div(class: 'mt-5 border-l-2 border-outline-variant pl-4') do
            m3_text(variant: :label_medium, class: 'block font-bold text-on-surface-variant') do
              t('medication_reviews.evidence_excerpt')
            end
            p(class: 'mt-2 text-sm leading-6 text-foreground') { prompt.evidence_text }
          end
          div(class: 'mt-4 flex flex-wrap gap-x-5 gap-y-2 text-xs text-on-surface-variant') do
            a(href: prompt.evidence_source_url, target: '_blank', rel: 'noopener noreferrer',
              class: 'font-bold text-primary underline underline-offset-2') { prompt.evidence_source_name }
            span { t('medication_reviews.checked_on', date: I18n.l(prompt.evidence_source_checked_on)) }
          end
          render_recorded_outcome if prompt.practitioner_review_status?
        end
      end

      def render_recorded_outcome
        div(class: 'mt-5 flex gap-3 border-t border-border pt-4') do
          render Icons::CheckCircle.new(size: 20, class: 'mt-0.5 shrink-0 text-primary')
          p(class: 'text-sm font-medium leading-6') do
            t(
              'medication_reviews.recorded_outcome',
              name: prompt.practitioner_name,
              role: prompt.practitioner_role,
              date: I18n.l(prompt.reviewed_on)
            )
          end
        end
      end

      def render_review_form
        render ReviewForm.new(prompt: prompt)
      end

      def risk_badge_variant
        prompt.risk_level == 'high' ? :destructive : :tonal
      end

      def risk_label
        t("medication_reviews.risk_levels.#{prompt.risk_level}")
      end

      def confidence_label
        t("medication_reviews.confidence_levels.#{prompt.match_confidence}")
      end

      def status_label
        t("medication_reviews.statuses.#{prompt.status}")
      end
    end
  end
end

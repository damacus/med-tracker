# frozen_string_literal: true

module Components
  module MedicationReviews
    class PromptCard < Components::Base
      def initialize(prompt:)
        @prompt = prompt
        super()
      end

      def view_template
        Card(class: 'overflow-hidden rounded-shape-lg bg-surface-container-lowest',
             data: { review_prompt_id: prompt.id }) do
          article(class: 'p-6 md:px-6 md:py-8') do
            Collapsible(open: prompt.errors.any?) do
              render_summary
              render_evidence
            end
          end
        end
      end

      private

      attr_reader :prompt

      def render_summary
        div(class: 'flex flex-col gap-5 md:flex-row md:items-center md:justify-between') do
          div(class: 'min-w-0') do
            m3_heading(level: 3, size: '5', class: 'font-bold') do
              "#{prompt.primary_medication_name} + #{prompt.interacting_medication_name}"
            end
            div(class: 'mt-2 flex flex-wrap items-center gap-2') do
              m3_badge(variant: priority_badge_variant) { priority_label }
              m3_badge(variant: :outlined) { status_label }
            end
            m3_text(variant: :body_medium, class: 'mt-3 block leading-relaxed text-on-surface-variant') do
              summary_text
            end
          end
          div(class: 'flex shrink-0 flex-col gap-2 sm:flex-row md:flex-col') do
            render_evidence_trigger
          end
        end
      end

      def render_evidence
        CollapsibleContent(
          class: "#{'hidden ' if prompt.errors.empty?}mt-5 border-t border-border pt-5",
          data: { testid: "prompt-evidence-#{prompt.id}" }
        ) do
          m3_text(variant: :label_medium, class: 'block font-bold text-on-surface-variant') do
            t('medication_reviews.evidence_excerpt')
          end
          p(class: 'mt-2 text-sm leading-6 text-foreground') { prompt.evidence_text }
          render_match_explanation
          div(class: 'mt-4 flex flex-wrap gap-x-5 gap-y-2 text-xs text-on-surface-variant') do
            a(href: prompt.evidence_source_url, target: '_blank', rel: 'noopener noreferrer',
              class: 'font-bold text-primary underline underline-offset-2') { prompt.evidence_source_name }
            span { t('medication_reviews.checked_on', date: I18n.l(prompt.evidence_source_checked_on)) }
          end
          render_recorded_outcome if prompt.practitioner_review_status?
          div(class: 'mt-6 border-t border-border pt-5', data: { testid: "prompt-review-form-#{prompt.id}" }) do
            render ReviewForm.new(prompt: prompt)
          end
        end
      end

      def render_evidence_trigger
        CollapsibleTrigger do
          m3_button(type: :button, variant: :outlined, class: 'w-full gap-2') do
            render Icons::FileText.new(size: 18)
            plain t('medication_reviews.view_evidence')
          end
        end
      end

      def render_match_explanation
        div(class: 'mt-4 border-y border-border py-4') do
          m3_text(variant: :label_medium, class: 'block font-bold text-on-surface-variant') do
            t('medication_reviews.match_explanation')
          end
          p(class: 'mt-2 text-sm leading-6 text-foreground') { prompt.match_reason }
          div(class: 'mt-2 grid gap-1 text-xs text-on-surface-variant') do
            span do
              t('medication_reviews.matched_term', term: prompt.matched_term, type: match_type_label)
            end
            span do
              t('medication_reviews.source_instruction', value: source_instruction_label)
            end
            span do
              t('medication_reviews.label_version', version: prompt.evidence_source_version,
                                                    date: I18n.l(prompt.evidence_source_effective_on,
                                                                 format: '%-d %B %Y'))
            end
          end
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

      def priority_label
        t("medication_reviews.filters.priorities.#{priority_key}")
      end

      def priority_key
        return 'low_confidence' if low_confidence?
        return 'discuss_soon' if prompt.risk_level == 'high'

        'ask_when_convenient'
      end

      def low_confidence?
        prompt.risk_level.in?(%w[low unknown]) || prompt.match_confidence.in?(%w[low unknown])
      end

      def priority_badge_variant
        case priority_key
        when 'discuss_soon' then :destructive
        when 'ask_when_convenient' then :warning
        else :tonal
        end
      end

      def summary_text
        t("medication_reviews.summaries.#{prompt.source_instruction}")
      end

      def status_label
        t("medication_reviews.statuses.#{prompt.status}")
      end

      def match_type_label
        t("medication_reviews.match_types.#{prompt.match_type}")
      end

      def source_instruction_label
        t("medication_reviews.source_instructions.#{prompt.source_instruction}")
      end
    end
  end
end

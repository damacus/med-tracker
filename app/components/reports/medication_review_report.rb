module Components
  module Reports
    class MedicationReviewReport < Phlex::HTML
      def initialize(prompts:)
        @prompts = prompts
        super()
      end

      def view_template
        summary
        boundary
        legend
        prompts.empty? ? empty_state : people
      end

      private

      attr_reader :prompts

      def summary
        section(class: 'report-section report-summary') do
          summary_item('to_discuss', prompts.count { |prompt| prompt.status == 'needs_review' })
          summary_item('reviewed', prompts.count(&:practitioner_review_status?))
          summary_item('people', prompts.map(&:person_id).uniq.size)
        end
      end

      def summary_item(key, count)
        div(class: 'summary-item') do
          strong { count.to_s }
          span { t("summary.#{key}") }
        end
      end

      def boundary
        section(class: 'callout') { p { t('boundary') } }
      end

      def legend
        section(class: 'report-section') do
          h2 { t('legend.title') }
          ul(class: 'risk-legend') do
            %w[high moderate low unknown].each do |risk_level|
              li(class: "risk-#{risk_level}") { t("legend.#{risk_level}") }
            end
          end
        end
      end

      def empty_state
        section(class: 'empty-state') do
          h2 { t('empty_title') }
          p { t('empty_description') }
        end
      end

      def people
        prompts.group_by(&:person).each do |person, person_prompts|
          section(class: 'report-section person-review') do
            h2 { person.name }
            p(class: 'person-review-count') { t('person_items', count: person_prompts.size) }
            person_prompts.each { |prompt| prompt_section(prompt) }
          end
        end
      end

      def prompt_section(prompt)
        article(class: 'report-section review-prompt') do
          prompt_heading(prompt)
          evidence(prompt)
          match_explanation(prompt)
          source(prompt)
          outcome(prompt)
        end
      end

      def prompt_heading(prompt)
        header(class: 'review-prompt-heading') do
          h3 { "#{prompt.primary_medication_name} + #{prompt.interacting_medication_name}" }
          p(class: "risk-#{prompt.risk_level}") { t("risk_levels.#{prompt.risk_level}") }
          p { t('confidence', value: prompt.match_confidence.capitalize) }
          p(class: 'review-status') { t("statuses.#{prompt.status}") }
        end
      end

      def evidence(prompt)
        section do
          h4 { t('evidence_title') }
          p { prompt.evidence_text.to_s.truncate(620) }
        end
      end

      def match_explanation(prompt)
        section do
          h4 { t('match_explanation_title') }
          p { prompt.match_reason }
          p do
            span { t('matched_term_name', term: prompt.matched_term) }
            plain ' '
            span(class: 'parenthetical') { t('matched_term_type', type: t("match_types.#{prompt.match_type}")) }
          end
          p { t('source_instruction', value: t("source_instructions.#{prompt.source_instruction}")) }
        end
      end

      def source(prompt)
        section do
          p { t('source', source: prompt.evidence_source_name) }
          p do
            t(
              'label_version',
              version: prompt.evidence_source_version,
              date: date(prompt.evidence_source_effective_on)
            )
          end
          p { t('retrieved', date: date(prompt.evidence_source_checked_on)) }
          a(href: prompt.evidence_source_url) { prompt.evidence_source_url }
        end
      end

      def outcome(prompt)
        section(class: 'callout review-outcome') do
          if prompt.practitioner_review_status?
            p do
              span { t('reviewed_with_name', name: prompt.practitioner_name) }
              plain ' '
              span(class: 'parenthetical') do
                t('reviewed_with_detail', role: prompt.practitioner_role, date: date(prompt.reviewed_on))
              end
            end
            p { t("outcomes.#{prompt.status}") }
            p { prompt.review_note } if prompt.review_note.present?
          else
            p { t('outstanding_outcome') }
          end
        end
      end

      def date(value)
        t('date_format', day: value.day, month: t('months').fetch(value.month - 1), year: value.year)
      end

      def t(key, **)
        I18n.t("reports.medication_review.#{key}", **)
      end
    end
  end
end

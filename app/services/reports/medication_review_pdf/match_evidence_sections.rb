# frozen_string_literal: true

module Reports
  class MedicationReviewPdf
    module MatchEvidenceSections
      def render_source(prompt)
        render_detail_lines(source_lines(prompt))
        document.text prompt.evidence_source_url, size: 7, color: COLORS.fetch(:muted),
                                                  link: prompt.evidence_source_url
        document.move_down 8
      end

      def render_match_explanation(prompt)
        document.text 'WHY MEDTRACKER INCLUDED THIS', size: 8, style: :bold, color: COLORS.fetch(:muted),
                                                      character_spacing: 0.7
        document.move_down 4
        render_detail_lines(match_explanation_lines(prompt))
        document.move_down 7
      end

      def render_detail_lines(lines)
        lines.each { |text, options| document.text(text, **options) }
      end

      def source_lines(prompt)
        [
          ["Source: #{prompt.evidence_source_name}", { size: 8, style: :bold, color: COLORS.fetch(:forest) }],
          ["Label version: #{prompt.evidence_source_version} | " \
           "Effective: #{date(prompt.evidence_source_effective_on)}", { size: 8, color: COLORS.fetch(:muted) }],
          ["Retrieved: #{date(prompt.evidence_source_checked_on)}", { size: 8, color: COLORS.fetch(:muted) }]
        ]
      end

      def match_explanation_lines(prompt)
        [
          [prompt.match_reason, { size: 8, color: COLORS.fetch(:ink) }],
          ["Matched term: #{prompt.matched_term} (#{pdf_match_type(prompt)} match)",
           { size: 8, color: COLORS.fetch(:muted) }],
          ["Source instruction category: #{pdf_source_instruction(prompt)}",
           { size: 8, color: COLORS.fetch(:muted) }]
        ]
      end

      def pdf_match_type(prompt)
        labels = { 'ingredient' => 'ingredient', 'class' => 'public class', 'curated' => 'curated',
                   'legacy' => 'legacy' }
        labels.fetch(prompt.match_type, prompt.match_type.humanize.downcase)
      end

      def pdf_source_instruction(prompt)
        labels = {
          'contraindicated' => 'Label says contraindicated',
          'avoid' => 'Label says avoid',
          'monitor_or_adjust' => 'Label says monitor or adjust',
          'possible_or_caution' => 'Label describes a possible concern or caution',
          'unclassified' => 'No instruction category assigned'
        }
        labels.fetch(prompt.source_instruction, prompt.source_instruction.humanize)
      end
    end
  end
end

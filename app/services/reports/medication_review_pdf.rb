# frozen_string_literal: true

require 'prawn'
require 'prawn/table'

module Reports
  class MedicationReviewPdf
    include OpeningSections

    COLORS = {
      ink: '17211F',
      muted: '5B6864',
      forest: '174A46',
      mint: 'DDEBE7',
      coral: 'B94B43',
      amber: 'B27718',
      sage: '4E725A',
      grey: '6B7280',
      line: 'D7DEDB',
      paper: 'F7F9F8',
      white: 'FFFFFF'
    }.freeze

    def initialize(prompts:, generated_at:)
      @prompts = prompts
      @generated_at = generated_at
    end

    delegate :render, to: :pdf

    private

    attr_reader :prompts, :generated_at, :document

    def pdf
      @pdf ||= Prawn::Document.new(page_size: 'A4', margin: 42, compress: false, info: document_info).tap do |pdf|
        @document = pdf
        render_document
      end
    end

    def render_document
      render_header
      render_summary
      render_boundary
      render_legend
      prompts.empty? ? render_empty : render_people
      render_page_numbers
    end

    def render_empty
      document.move_down 24
      document.text 'No medicine review items matched the selected filters.', size: 12, style: :bold,
                                                                              align: :center
      document.move_down 8
      document.text 'This can mean there are no reviewed evidence pairs for the active medicines in scope.',
                    size: 9, color: COLORS.fetch(:muted), align: :center
    end

    def render_people
      prompts.group_by(&:person).each do |person, person_prompts|
        render_person_heading(person)
        person_prompts.each { |prompt| render_prompt(prompt) }
      end
    end

    def render_person_heading(person)
      start_person_page
      render_person_heading_rule
      document.move_down 8
      document.text person.name, size: 17, style: :bold, color: COLORS.fetch(:forest)
      document.text person_prompts_label(person), size: 8, color: COLORS.fetch(:muted)
      document.move_down 12
    end

    def start_person_page
      document.start_new_page if document.cursor < 120
    end

    def render_person_heading_rule
      document.stroke_color COLORS.fetch(:forest)
      document.line_width 2
      document.stroke_horizontal_rule
    end

    def person_prompts_label(person)
      count = prompts.count { |prompt| prompt.person_id == person.id }
      "#{count} review #{count == 1 ? 'item' : 'items'}"
    end

    def render_prompt(prompt)
      render_continuation_heading(prompt) if document.cursor < 200
      render_prompt_heading(prompt)
      render_evidence(prompt)
      render_source(prompt)
      render_review_outcome(prompt)
      document.move_down 14
    end

    def render_continuation_heading(prompt)
      document.start_new_page
      document.text "#{prompt.person.name} - continued", size: 12, style: :bold, color: COLORS.fetch(:forest)
      document.move_down 10
    end

    def render_prompt_heading(prompt)
      render_prompt_heading_table(prompt)
      document.move_down 8
      document.text status_label(prompt), size: 8, style: :bold, color: COLORS.fetch(:muted)
      document.move_down 8
    end

    def render_prompt_heading_table(prompt)
      data = [[medicine_pair(prompt), risk_label(prompt)]]
      widths = [document.bounds.width - 92, 92]
      document.table(data, width: document.bounds.width, column_widths: widths,
                           cell_style: { padding: [9, 10], border_color: COLORS.fetch(:line) }) do |table|
        style_prompt_heading(table, prompt)
      end
    end

    def style_prompt_heading(table, prompt)
      table.cells.background_color = COLORS.fetch(:paper)
      style_prompt_name(table.column(0))
      style_risk_label(table.column(1), prompt)
    end

    def style_prompt_name(cell)
      cell.font_style = :bold
      cell.size = 12
    end

    def style_risk_label(cell, prompt)
      cell.font_style = :bold
      cell.size = 8
      cell.align = :center
      cell.text_color = risk_color(prompt.risk_level)
    end

    def render_evidence(prompt)
      document.text 'PUBLIC LABEL EVIDENCE', size: 8, style: :bold, color: COLORS.fetch(:muted),
                                             character_spacing: 0.7
      document.move_down 4
      document.text prompt.evidence_text.to_s.truncate(620), size: 9, leading: 2, color: COLORS.fetch(:ink)
      document.move_down 7
    end

    def render_source(prompt)
      document.text "Source: #{prompt.evidence_source_name}", size: 8, style: :bold,
                                                              color: COLORS.fetch(:forest)
      document.text "Checked: #{date(prompt.evidence_source_checked_on)}", size: 8, color: COLORS.fetch(:muted)
      document.text prompt.evidence_source_url, size: 7, color: COLORS.fetch(:muted),
                                                link: prompt.evidence_source_url
      document.move_down 8
    end

    def render_review_outcome(prompt)
      return render_outstanding_outcome unless prompt.practitioner_review_status?

      lines = [
        "Reviewed with #{prompt.practitioner_name} (#{prompt.practitioner_role}) on #{date(prompt.reviewed_on)}.",
        outcome_label(prompt)
      ]
      lines << prompt.review_note if prompt.review_note.present?
      outcome_box(lines.join("\n"), COLORS.fetch(:mint), COLORS.fetch(:forest))
    end

    def render_outstanding_outcome
      outcome_box('No practitioner review has been recorded yet.', COLORS.fetch(:paper), COLORS.fetch(:muted))
    end

    def outcome_box(text, background, foreground)
      document.table([[text]], width: document.bounds.width, cell_style: { padding: 9 }) do
        cells.background_color = background
        cells.border_color = COLORS.fetch(:line)
        cells.text_color = foreground
        cells.size = 8
      end
    end

    def render_page_numbers
      document.number_pages 'Page <page> of <total>', at: [0, -20], width: document.bounds.width,
                                                      align: :right, size: 8, color: COLORS.fetch(:muted)
    end

    def generated_label
      "Prepared #{generated_at.in_time_zone.strftime('%-d %B %Y at %H:%M %Z')}"
    end

    def medicine_pair(prompt)
      "#{prompt.primary_medication_name} + #{prompt.interacting_medication_name}"
    end

    def risk_label(prompt)
      "#{prompt.risk_level.upcase} SOURCE RISK\n#{prompt.match_confidence.capitalize} match confidence"
    end

    def risk_color(risk_level)
      { 'high' => COLORS.fetch(:coral), 'moderate' => COLORS.fetch(:amber),
        'low' => COLORS.fetch(:sage), 'unknown' => COLORS.fetch(:grey) }.fetch(risk_level)
    end

    def status_label(prompt)
      "STATUS: #{prompt.status.humanize.upcase}"
    end

    def outcome_label(prompt)
      return 'Recorded outcome: expected as prescribed.' if prompt.status == 'expected_prescribed_combination'

      'Recorded outcome: reviewed with practitioner.'
    end

    def boundary_text
      'This record organises public medicine-label evidence for discussion with a practitioner. ' \
        'It does not replace clinical judgement or tell someone to change a medicine.'
    end

    def date(value)
      value.strftime('%-d %B %Y')
    end

    def document_info
      { Title: 'MedTracker medicine review record', Creator: 'MedTracker', Subject: boundary_text }
    end
  end
end

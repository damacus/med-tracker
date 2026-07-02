# frozen_string_literal: true

require 'prawn'
require 'prawn/table'

module Reports
  class HealthHistoryPdf
    attr_reader :result, :start_date, :end_date, :generated_at

    def initialize(result:, start_date:, end_date:, generated_at:)
      @result = result
      @start_date = start_date
      @end_date = end_date
      @generated_at = generated_at
    end

    delegate :render, to: :pdf

    private

    def pdf
      @pdf ||= Prawn::Document.new(page_size: 'A4', margin: 36, compress: false, info: document_info).tap do |document|
        render_document(document)
      end
    end

    def render_document(document)
      @document = document
      render_header
      render_medication_takes
      render_suspected_side_effects
      render_notable_illnesses
      render_illness_patterns
      render_disclaimer
    end

    def render_header
      heading(t('title'), size: 22)
      header_lines.each { |line| text(line) }
      document.move_down 18
    end

    def header_lines
      [
        t('people', people: people_label),
        t('date_range', start_date: date(start_date), end_date: date(end_date)),
        t('generated_at', timestamp: generated_at.in_time_zone.strftime('%Y-%m-%d %H:%M %Z'))
      ]
    end

    def render_medication_takes
      section(t('medication_takes.title')) do
        table_or_empty(result.medication_takes, medication_take_headings) { |take| medication_take_row(take) }
      end
    end

    def medication_take_headings
      %w[time person medication dose source location].map { |key| t("medication_takes.#{key}") }
    end

    def medication_take_row(take)
      [
        take.taken_at.in_time_zone.strftime('%Y-%m-%d %H:%M'),
        take.person.name,
        take.medication_name,
        take.dose_display,
        t("sources.#{take.source_type}"),
        take.location_name.to_s
      ]
    end

    def render_suspected_side_effects
      section(t('suspected_side_effects.title')) do
        event_table(result.suspected_side_effects, include_medications: true)
      end
    end

    def render_notable_illnesses
      section(t('notable_illnesses.title')) do
        event_table(result.notable_illnesses, include_medications: false)
      end
    end

    def render_illness_patterns
      section(t('illness_patterns.title')) do
        render_pattern_summaries
      end
    end

    def render_pattern_summaries
      return text(t('empty_section')) if result.illness_patterns.empty?

      result.illness_patterns.each { |pattern| text(pattern_summary(pattern)) }
    end

    def pattern_summary(pattern)
      t(
        'illness_patterns.summary',
        count: pattern.episode_count,
        title: pattern.display_title,
        first: date(pattern.first_started_on),
        last: date(pattern.most_recent_started_on),
        interval: pattern.average_interval_days
      )
    end

    def event_table(events, include_medications:)
      table_or_empty(events, event_headings(include_medications)) do |event|
        event_row(event, include_medications)
      end
    end

    def event_headings(include_medications)
      headings = %w[title dates severity notes action].map { |key| t("events.#{key}") }
      headings.insert(3, t('events.medications')) if include_medications
      headings
    end

    def event_row(event, include_medications)
      row = [event.title, event_date_range(event), event.severity.to_s.humanize, event.notes.to_s,
             event.action_taken.to_s]
      row.insert(3, event.medication_names.to_sentence) if include_medications
      row
    end

    def table_or_empty(records, headings, &)
      return text(t('empty_section')) if records.empty?

      document.table([headings] + records.map(&), header: true, width: document.bounds.width) do
        row(0).font_style = :bold
        cells.size = 9
        cells.padding = 6
      end
    end

    def render_disclaimer
      section(t('disclaimer.title')) do
        text(t('disclaimer.entered_information'))
        text(t('disclaimer.causation'))
        text(t('disclaimer.medical_advice'))
      end
    end

    def section(title)
      heading(title, size: 15)
      yield
      document.move_down 16
    end

    def heading(value, size:)
      document.text(value, size: size, style: :bold)
      document.move_down 8
    end

    def text(value)
      document.text(value.to_s, size: 10, leading: 2)
      document.move_down 4
    end

    def event_date_range(event)
      return t('ongoing_from', started_on: date(event.started_on)) if event.ongoing?

      t('event_date_range', started_on: date(event.started_on), ended_on: date(event.ended_on))
    end

    def people_label
      result.people.map(&:name).presence&.to_sentence || t('no_people')
    end

    def date(value)
      I18n.l(value)
    end

    def document_info
      { Title: t('title'), Creator: 'MedTracker' }
    end

    def t(key, **)
      I18n.t("reports.health_history.#{key}", **)
    end

    attr_reader :document
  end
end

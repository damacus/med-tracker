# frozen_string_literal: true

module Components
  module Reports
    class HealthHistoryReport < Phlex::HTML
      def initialize(result:)
        @result = result
        super()
      end

      def view_template
        medication_takes_section
        not_taken_section
        dose_outcomes_section
        cycle_outcomes_section
        suspected_side_effects_section
        notable_illnesses_section
        illness_patterns_section
        disclaimer_section
      end

      private

      attr_reader :result

      def medication_takes_section
        report_section(t('medication_takes.title')) do
          table_or_empty(
            result.medication_takes,
            medication_take_headings,
            table_class: 'health-history-medication-table'
          ) do |take|
            medication_take_row(take)
          end
        end
      end

      def suspected_side_effects_section
        report_section(t('suspected_side_effects.title')) do
          event_table(result.suspected_side_effects, include_medications: true)
        end
      end

      def not_taken_section
        report_section(t('outcomes.not_taken')) do
          headings = %w[time person medication].map { |key| t("medication_takes.#{key}") }
          headings.push(t('outcomes.reason'), t('events.notes'))
          table_or_empty(result.not_taken_outcomes, headings, table_class: 'health-history-not-taken-table') do |entry|
            not_taken_row(entry)
          end
        end
      end

      def not_taken_row(entry)
        reason = I18n.t("dashboard.outcomes.reasons.#{entry.reason}") if entry.reason.present?
        time = entry.scheduled_at&.in_time_zone&.strftime('%Y-%m-%d %H:%M') || date(entry.date)
        [time, entry.person.name, entry.medication_name, reason, entry.note]
      end

      def dose_outcomes_section
        report_section(t('outcomes.title')) do
          headings = %w[date expected taken not_taken unexplained_missed].map { |key| t("outcomes.#{key}") }
          table_or_empty(result.daily_outcomes, headings, table_class: 'health-history-outcomes-table') do |day|
            [date(day[:date]), day[:expected], day[:actual], day[:not_taken], day[:unexplained_missed]]
          end
        end
      end

      def notable_illnesses_section
        report_section(t('notable_illnesses.title')) do
          event_table(result.notable_illnesses, include_medications: false)
        end
      end

      def cycle_outcomes_section
        return if result.cycle_summaries.empty?

        report_section(t('cycles.title')) do
          p { t('cycles.description') }
          headings = [t('cycles.window'), t('medication_takes.person'), t('medication_takes.medication')]
          headings.concat(%w[expected taken not_taken unexplained_missed].map { |key| t("outcomes.#{key}") })
          table_or_empty(result.cycle_summaries, headings, table_class: 'health-history-cycles-table') do |cycle|
            cycle_row(cycle)
          end
        end
      end

      def cycle_row(cycle)
        window = t('event_date_range', started_on: date(cycle[:window_starts_on]),
                                       ended_on: date(cycle[:window_ends_on]))
        [window, cycle[:source].person.name, cycle[:source].medication.display_name,
         cycle[:expected], cycle[:actual], cycle[:not_taken], cycle[:unexplained_missed]]
      end

      def illness_patterns_section
        report_section(t('illness_patterns.title')) do
          if result.illness_patterns.empty?
            empty_state
          else
            result.illness_patterns.each { |pattern| p { pattern_summary(pattern) } }
          end
        end
      end

      def disclaimer_section
        report_section(t('disclaimer.title'), class_name: 'callout') do
          p { t('disclaimer.entered_information') }
          p { t('disclaimer.causation') }
          p { t('disclaimer.medical_advice') }
        end
      end

      def report_section(title, class_name: 'report-section')
        section(class: class_name) do
          h2 { title }
          yield
        end
      end

      def table_or_empty(records, headings, table_class:)
        return empty_state if records.empty?

        table(class: table_class) do
          thead do
            tr { headings.each { |heading| th { heading } } }
          end
          tbody do
            records.each do |record|
              tr { yield(record).each { |value| td { value.to_s } } }
            end
          end
        end
      end

      def empty_state
        div(class: 'empty-state') { p { t('empty_section') } }
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

      def event_table(events, include_medications:)
        table_class = include_medications ? 'health-history-side-effects-table' : 'health-history-illnesses-table'
        table_or_empty(events, event_headings(include_medications), table_class:) do |event|
          event_row(event, include_medications)
        end
      end

      def event_headings(include_medications)
        headings = %w[title dates severity notes action].map { |key| t("events.#{key}") }
        headings.insert(3, t('events.medications')) if include_medications
        headings
      end

      def event_row(event, include_medications)
        row = [
          event.title,
          event_date_range(event),
          event.severity.to_s.humanize,
          event.notes.to_s,
          event.action_taken.to_s
        ]
        row.insert(3, event.medication_names.to_sentence) if include_medications
        row
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

      def event_date_range(event)
        return t('ongoing_from', started_on: date(event.started_on)) if event.ongoing?

        t('event_date_range', started_on: date(event.started_on), ended_on: date(event.ended_on))
      end

      def date(value)
        I18n.l(value)
      end

      def t(key, **)
        I18n.t("reports.health_history.#{key}", **)
      end
    end
  end
end

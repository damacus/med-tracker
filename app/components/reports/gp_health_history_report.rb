# frozen_string_literal: true

module Components
  module Reports
    class GpHealthHistoryReport < Phlex::HTML
      def initialize(result:, include_medication_takes: false)
        @result = result
        @include_medication_takes = include_medication_takes
        super()
      end

      def view_template
        identity_section
        current_medicines_section
        chronology_section
        disclaimer_section
        medication_takes_appendix if @include_medication_takes
      end

      private

      attr_reader :result

      def identity_section
        report_section(t('gp.identity.title')) do
          p { t('gp.identity.date_of_birth', date: date(result.person.date_of_birth)) }
        end
      end

      def current_medicines_section
        report_section(t('gp.current_medicines.title')) do
          if result.current_medicines.empty?
            empty_state
          else
            ul { result.current_medicines.each { |medicine| li { medicine.display_name } } }
          end
        end
      end

      def chronology_section
        report_section(t('gp.chronology.title')) do
          return empty_state if result.chronology.empty?

          table(class: 'health-history-chronology-table') do
            thead do
              tr { chronology_headings.each { |heading| th { heading } } }
            end
            tbody do
              result.chronology.each { |entry| chronology_entry(entry) }
            end
          end
        end
      end

      def chronology_entry(entry)
        tr do
          td { event_date_range(entry) }
          td { event_kind(entry) }
          td { entry.title }
          td do
            ul { chronology_details(entry).each { |detail| li { detail } } }
          end
        end
      end

      def chronology_headings
        %w[dates title].map { |key| t("events.#{key}") }.insert(1, t('gp.chronology.kind_heading')).append(
          t('gp.chronology.details_heading')
        )
      end

      def medication_takes_appendix
        section(class: 'pdf-appendix') do
          h2 { t('gp.medication_takes_appendix') }
          table_or_empty(result.medication_takes, medication_take_headings) do |take|
            medication_take_row(take)
          end
        end
      end

      def table_or_empty(records, headings)
        return empty_state if records.empty?

        table(class: 'health-history-medication-table') do
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

      def chronology_details(entry)
        [
          t('gp.chronology.duration', duration: chronology_duration(entry)),
          severity_detail(entry),
          notes_detail(entry),
          action_detail(entry),
          t('gp.chronology.medical_help', medical_help: medical_help(entry)),
          medication_detail(entry)
        ].compact
      end

      def severity_detail(entry)
        return if entry.severity.blank?

        t('gp.chronology.severity', severity: I18n.t("health_events.severities.#{entry.severity}"))
      end

      def notes_detail(entry)
        t('gp.chronology.notes', notes: entry.notes) if entry.notes.present?
      end

      def action_detail(entry)
        t('gp.chronology.action', action: entry.action_taken) if entry.action_taken.present?
      end

      def medication_detail(entry)
        return if entry.medication_names.blank?

        t('gp.chronology.medications', medications: entry.medication_names.to_sentence(**t('gp.list').symbolize_keys))
      end

      def chronology_duration(entry)
        return t('gp.chronology.ongoing') if entry.ongoing?

        t('gp.chronology.days', count: entry.duration_days)
      end

      def medical_help(entry)
        entry.medical_help_sought ? t('gp.yes') : t('gp.no')
      end

      def event_kind(entry)
        I18n.t("health_events.kinds.#{entry.event_kind}")
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

      def empty_state
        div(class: 'empty-state') { p { t('empty_section') } }
      end

      def event_date_range(entry)
        return t('ongoing_from', started_on: date(entry.started_on)) if entry.ongoing?

        t('event_date_range', started_on: date(entry.started_on), ended_on: date(entry.ended_on))
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

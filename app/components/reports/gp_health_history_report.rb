# frozen_string_literal: true

module Components
  module Reports
    class GpHealthHistoryReport < Phlex::HTML
      def initialize(result:)
        @result = result
        super()
      end

      def view_template
        identity_section
        current_medicines_section
        chronology_section
        disclaimer_section
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

          result.chronology.each { |entry| chronology_entry(entry) }
        end
      end

      def chronology_entry(entry)
        article(class: 'health-history-chronology-entry') do
          h3 { "#{entry.title} — #{event_date_range(entry)}" }
          ul do
            chronology_details(entry).each { |detail| li { detail } }
          end
        end
      end

      def chronology_details(entry)
        [
          t('gp.chronology.duration', duration: chronology_duration(entry)),
          t('gp.chronology.severity', severity: entry.severity.to_s.humanize.presence || t('gp.not_recorded')),
          t('gp.chronology.notes', notes: entry.notes.presence || t('gp.not_recorded')),
          t('gp.chronology.action', action: entry.action_taken.presence || t('gp.not_recorded')),
          t('gp.chronology.medical_help', medical_help: medical_help(entry)),
          t('gp.chronology.medications', medications: entry.medication_names.presence&.to_sentence || t('gp.none'))
        ]
      end

      def chronology_duration(entry)
        return t('gp.chronology.ongoing') if entry.ongoing?

        t('gp.chronology.days', count: entry.duration_days)
      end

      def medical_help(entry)
        entry.medical_help_sought ? t('gp.yes') : t('gp.no')
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

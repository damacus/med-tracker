module Components
  module MedicationPauses
    class History < Components::Base
      def initialize(periods:)
        @periods = periods.sort_by { |period| [period.created_at, period.id.to_i] }.reverse
        super()
      end

      def view_template
        return if periods.empty?

        div(class: 'px-8 pb-6 space-y-3') do
          active = periods.find { |period| period.ended_at.nil? }
          render_period(active) if active
          completed_periods = periods.select(&:ended_at)
          details(class: 'rounded-xl border border-border p-3') do
            summary(class: 'cursor-pointer font-medium focus-visible:ring-2 focus-visible:ring-primary') do
              t('medication_pauses.history')
            end
            ol(class: 'space-y-4 pt-3') do
              completed_periods.each { |period| li { render_period(period) } }
            end
          end
        end
      end

      private

      attr_reader :periods

      def render_period(period)
        div(class: 'space-y-1 text-sm') do
          Text(class: 'font-semibold') { t("medication_pauses.reasons.#{period.reason}") }
          Text(class: 'whitespace-pre-wrap break-words') { period.note } if period.note.present?
          Text do
            plain t('medication_pauses.started', date: date_label(period.started_at),
                                                 actor: actor_label(period.recorded_by_membership))
          end
          if period.ended_at
            Text do
              plain t('medication_pauses.resumed', date: date_label(period.ended_at),
                                                   actor: actor_label(period.resumed_by_membership))
            end
          end
        end
      end

      def date_label(value)
        value ? I18n.l(value, format: :long) : t('medication_pauses.start_unknown')
      end

      def actor_label(membership)
        membership&.person&.name || t('medication_pauses.actor_unknown')
      end
    end
  end
end

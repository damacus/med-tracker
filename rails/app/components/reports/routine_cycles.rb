module Components
  module Reports
    class RoutineCycles < Components::Base
      def initialize(cycles:)
        @cycles = cycles
        super()
      end

      def view_template
        return if @cycles.empty?

        section(id: 'routine-cycles', class: 'space-y-5') do
          m3_heading(level: 2, size: '5') { translate('cycles.title') }
          m3_text(size: '2', class: 'text-on-surface-variant') { translate('cycles.description') }
          @cycles.each { |cycle| cycle_card(cycle) }
        end
      end

      private

      def cycle_card(cycle)
        m3_card(class: 'border border-border/70 bg-card p-5 space-y-4') do
          m3_heading(level: 3, size: '3') { cycle[:source].medication.display_name }
          m3_text(size: '2') { cycle[:source].person.name }
          m3_text(size: '2', class: 'text-on-surface-variant') do
            translate('event_date_range', started_on: cycle[:window_starts_on].iso8601,
                                          ended_on: cycle[:window_ends_on].iso8601)
          end
          dl(class: 'grid grid-cols-2 gap-4 sm:grid-cols-4') do
            { expected: :expected, taken: :actual, not_taken: :not_taken,
              unexplained_missed: :unexplained_missed }.each do |label, field|
              cycle_count(label, cycle.fetch(field))
            end
          end
        end
      end

      def cycle_count(label, count)
        div do
          dt(class: 'text-sm text-on-surface-variant') { translate("outcomes.#{label}") }
          dd(class: 'text-xl font-semibold') { count.to_s }
        end
      end

      def translate(key, **)
        I18n.t("reports.health_history.#{key}", **)
      end
    end
  end
end

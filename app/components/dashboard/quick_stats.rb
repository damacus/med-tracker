# frozen_string_literal: true

module Components
  module Dashboard
    # Renders the quick stats table on the dashboard
    class QuickStats < Components::Base
      attr_reader :people, :active_prescriptions

      def initialize(people:, active_prescriptions:)
        @people = people
        @active_prescriptions = active_prescriptions
        super()
      end

      def view_template
        section(class: 'dashboard__section dashboard__section--stats') do
          Table(class: 'dashboard__stats-table') do
            caption(class: 'dashboard__section-title text-left') { 'Quick stats' }
            TableBody do
              render_stat_row('People', people.count)
              render_stat_row('Active prescriptions', active_prescriptions.count)
            end
          end
        end
      end

      private

      def render_stat_row(label, value)
        TableRow do
          th(scope: 'row', class: 'dashboard__stats-label') { "#{label}:" }
          TableCell(class: 'dashboard__stats-value') { value }
        end
      end
    end
  end
end

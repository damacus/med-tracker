# frozen_string_literal: true

module Components
  module Dashboard
    # Renders the medication schedule section of the dashboard
    class Schedule < Components::Base
      attr_reader :people, :upcoming_prescriptions, :url_helpers

      def initialize(people:, upcoming_prescriptions:, url_helpers: nil)
        @people = people
        @upcoming_prescriptions = upcoming_prescriptions
        @url_helpers = url_helpers
        super()
      end

      def view_template
        section(class: 'dashboard__section') do
          h2(class: 'dashboard__section-title') { 'Medication Schedule By Person' }
          render_schedule_section
        end
      end

      private

      def render_schedule_section
        div(class: 'schedule-section') do
          if people.empty?
            return render_empty_state(
              'No people found',
              'Add people to start tracking their medications'
            )
          end
          if upcoming_prescriptions.empty?
            return render_empty_state(
              'No active prescriptions found',
              'Add prescriptions to see them here'
            )
          end

          render_schedule_content
        end
      end

      def render_schedule_content
        div(class: 'schedule-content') do
          upcoming_prescriptions.each do |person, prescriptions|
            render_schedule_for(person, prescriptions)
          end
        end
      end

      def render_schedule_for(person, prescriptions)
        render Components::Dashboard::PersonSchedule.new(
          person: person,
          prescriptions: prescriptions,
          take_medicine_url_generator: take_medicine_url_generator
        )
      end

      def take_medicine_url_generator
        return unless url_helpers

        ->(prescription) { url_helpers.prescription_take_medicines_path(prescription) }
      end

      def render_empty_state(message, help_text)
        div(class: 'empty-state') do
          p(class: 'empty-state__message') { message }
          p(class: 'empty-state__help') { help_text }
        end
      end
    end
  end
end

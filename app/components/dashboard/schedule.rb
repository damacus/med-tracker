# frozen_string_literal: true

module Components
  module Dashboard
    # Renders the medication schedule section of the dashboard
    class Schedule < Components::Base
      attr_reader :people, :upcoming_prescriptions, :current_user

      def initialize(people:, upcoming_prescriptions:, current_user: nil)
        @people = people
        @upcoming_prescriptions = upcoming_prescriptions
        @current_user = current_user
        super()
      end

      def view_template
        div(class: 'space-y-6') do
          Heading(level: 2) { 'Medication Schedule' }
          render_schedule_section
        end
      end

      private

      def render_schedule_section
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

      def render_schedule_content
        div(class: 'space-y-6') do
          upcoming_prescriptions.each do |person, prescriptions|
            render_schedule_for(person, prescriptions)
          end
        end
      end

      def render_schedule_for(person, prescriptions)
        render Components::Dashboard::PersonSchedule.new(
          person: person,
          prescriptions: prescriptions,
          take_medicine_url_generator: take_medicine_url_generator,
          current_user: current_user
        )
      end

      def take_medicine_url_generator
        ->(prescription) { prescription_medication_takes_path(prescription) }
      end

      def render_empty_state(message, help_text)
        Card(class: 'text-center py-12') do
          CardContent do
            Text(size: '5', weight: 'semibold', class: 'text-slate-700 mb-2') { message }
            Text(class: 'text-slate-600') { help_text }
          end
        end
      end
    end
  end
end

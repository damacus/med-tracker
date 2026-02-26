# frozen_string_literal: true

module Components
  module Schedules
    # Edit schedule view component
    class EditView < Components::Base
      attr_reader :schedule, :person, :medications

      def initialize(schedule:, person:, medications:)
        @schedule = schedule
        @person = person
        @medications = medications
        super()
      end

      def view_template
        div(class: 'container mx-auto px-4 py-8 max-w-4xl') do
          render_header
          render Form.new(schedule: schedule, person: person, medications: medications)
        end
      end

      private

      def render_header
        div(class: 'mb-8') do
          Text(size: '2', weight: 'medium', class: 'uppercase tracking-wide text-slate-500 mb-2') do
            'Edit Schedule'
          end
          Heading(level: 1) { "Update schedule for #{person.name}" }
        end
      end
    end
  end
end

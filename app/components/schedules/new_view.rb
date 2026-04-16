# frozen_string_literal: true

module Components
  module Schedules
    # New schedule view component
    class NewView < Components::Base
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
        div(class: 'mb-8 space-y-2') do
          m3_text(variant: :label_medium, class: 'uppercase tracking-[0.2em] font-black opacity-40') do
            'New Schedule'
          end
          m3_heading(variant: :display_small, level: 1, class: 'font-black tracking-tight text-foreground') do
            "Add schedule for #{person.name}"
          end
        end
      end
    end
  end
end

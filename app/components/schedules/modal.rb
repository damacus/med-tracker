# frozen_string_literal: true

module Components
  module Schedules
    # Modal component for schedule form using RubyUI Dialog
    class Modal < Components::Base
      attr_reader :schedule, :person, :medications, :title

      def initialize(schedule:, person:, medications:, title: nil)
        @schedule = schedule
        @person = person
        @medications = medications
        @title = title || "New Schedule for #{person.name}"
        super()
      end

      def view_template
        Dialog(open: true) do
          DialogContent(size: :xl) do
            DialogHeader do
              DialogTitle { title }
              DialogDescription { 'Add medication details and schedule' }
            end
            DialogMiddle do
              render Form.new(schedule: schedule, person: person, medications: medications)
            end
          end
        end
      end
    end
  end
end

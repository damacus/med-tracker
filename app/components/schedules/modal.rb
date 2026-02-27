# frozen_string_literal: true

module Components
  module Schedules
    # Modal component for schedule form using RubyUI Dialog
    class Modal < Components::Base
      include Phlex::Rails::Helpers::TurboFrameTag

      attr_reader :schedule, :person, :medications, :title

      def initialize(schedule:, person:, medications:, title: nil)
        @schedule = schedule
        @person = person
        @medications = medications
        @title = title || "New Schedule for #{person.name}"
        super()
      end

      def view_template
        turbo_frame_tag 'modal' do
          render ::Components::Modal.new(title: title, subtitle: 'Add medication details and schedule') do
            render Form.new(schedule: schedule, person: person, medications: medications)
          end
        end
      end
    end
  end
end

# frozen_string_literal: true

module Components
  module Schedules
    # Modal component for schedule form using RubyUI Dialog
    class Modal < Components::Base
      include Phlex::Rails::Helpers::TurboFrameTag
      include RubyUI

      attr_reader :schedule, :person, :medications, :title, :back_path, :editing

      def initialize(schedule:, person:, medications:, title: nil, back_path: nil, editing: false)
        @schedule = schedule
        @person = person
        @medications = medications
        @title = title || "New Schedule for #{person.name}"
        @back_path = back_path
        @editing = editing
        super()
      end

      def view_template
        turbo_frame_tag 'modal' do
          Dialog(open: true) do
            DialogContent(size: :xl) do
              DialogHeader do
                if back_path
                  a(
                    href: back_path,
                    data: { turbo_frame: 'modal' },
                    class: 'inline-flex items-center text-sm text-muted-foreground hover:text-foreground ' \
                           'transition-colors mb-2 no-underline'
                  ) do
                    plain t('medication_workflow.back')
                  end
                end
                DialogTitle { title }
                DialogDescription { t('schedules.modal.subtitle') }
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
end

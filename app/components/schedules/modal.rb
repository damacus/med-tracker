# frozen_string_literal: true

module Components
  module Schedules
    # Modal component for schedule form using RubyUI Dialog
    class Modal < Components::Base
      include Phlex::Rails::Helpers::TurboFrameTag
      include RubyUI

      attr_reader :schedule, :person, :medications, :title, :back_path, :editing

      def initialize(schedule:, person:, medications:, **options)
        @schedule = schedule
        @person = person
        @medications = medications
        title = options[:title]
        back_path = options[:back_path]
        editing = options.fetch(:editing, false)
        @title = title || "New Schedule for #{person.name}"
        @back_path = back_path
        @editing = editing
        super()
      end

      def view_template
        turbo_frame_tag 'modal' do
          Dialog(open: true) do
            DialogContent(
              size: :xl,
              class: 'overflow-hidden border-border/50 bg-white shadow-[0_32px_90px_rgba(15,23,42,0.18)]'
            ) do
              DialogHeader(class: 'bg-gradient-to-b from-[#fffaf1] to-white px-8 pt-8 pb-4') do
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
              DialogMiddle(class: 'bg-[#fffdf8] px-8 pb-8 pt-4') do
                render Form.new(schedule: schedule, person: person, medications: medications, frame_id: 'modal')
              end
            end
          end
        end
      end
    end
  end
end

# frozen_string_literal: true

module Views
  module Profiles
    class NestedSection < Views::Base
      def initialize(key:, title:, description:)
        super()
        @key = key
        @title = title
        @description = description
      end

      def view_template(&)
        render RubyUI::AccordionItem.new(class: 'rounded-shape-lg border border-border/70 bg-popover') do
          render_trigger
          render RubyUI::AccordionContent.new(
            id: content_id,
            labelledby: trigger_id,
            open: false,
            class: 'px-3 pb-3 sm:px-4 sm:pb-4',
            &
          )
        end
      end

      private

      def render_trigger
        render RubyUI::AccordionTrigger.new(
          controls: content_id,
          expanded: false,
          id: trigger_id,
          class: 'flex min-h-16 w-full items-center justify-between gap-4 px-4 py-3 text-left',
          data: { testid: 'profile-nested-setting' }
        ) do
          div(class: 'min-w-0') do
            p(class: 'font-semibold text-foreground') { @title }
            p(class: 'mt-1 text-sm leading-5 text-on-surface-variant') { @description }
          end
          render RubyUI::AccordionIcon.new
        end
      end

      def trigger_id
        "profile-#{@key}-trigger"
      end

      def content_id
        "profile-#{@key}-content"
      end
    end
  end
end

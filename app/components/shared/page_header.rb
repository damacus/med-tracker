# frozen_string_literal: true

module Components
  module Shared
    # Shared page header component that prioritizes quick actions for mobile
    # On mobile: actions appear first (sticky), then title
    # On desktop: title on left, actions on right
    class PageHeader < Components::Base
      attr_reader :title, :subtitle

      def initialize(title:, subtitle: nil)
        @title = title
        @subtitle = subtitle
        super()
      end

      def view_template(&)
        div(class: 'page-header mb-6 md:mb-8') do
          # Mobile layout: actions first (sticky at top for easy thumb access)
          div(class: 'md:hidden') do
            render_mobile_layout(&)
          end

          # Desktop layout: title left, actions right
          div(class: 'hidden md:block') do
            render_desktop_layout(&)
          end
        end
      end

      private

      def render_mobile_layout
        # Quick actions at top - sticky for easy access
        if block_given?
          div(class: 'sticky top-16 z-30 bg-background/95 backdrop-blur py-3 -mx-4 px-4 border-b mb-4') do
            div(class: 'flex flex-wrap gap-2') do
              yield :actions
            end
          end
        end

        # Title and subtitle below
        div(class: 'space-y-1') do
          Heading(level: 1, class: 'text-2xl') { title }
          render_subtitle if subtitle
        end
      end

      def render_desktop_layout
        div(class: 'flex flex-col sm:flex-row sm:justify-between sm:items-center gap-4') do
          div(class: 'space-y-1') do
            Heading(level: 1) { title }
            render_subtitle if subtitle
          end

          if block_given?
            div(class: 'flex flex-wrap gap-3') do
              yield :actions
            end
          end
        end
      end

      def render_subtitle
        Text(class: 'text-muted-foreground') { subtitle }
      end
    end
  end
end

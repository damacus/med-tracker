# frozen_string_literal: true

module Components
  module Dashboard
    # Renders the dashboard quick actions section
    class QuickActions < Components::Base
      attr_reader :url_helpers

      def initialize(url_helpers: nil)
        @url_helpers = url_helpers
        super()
      end

      def view_template
        section(class: 'dashboard__section dashboard__section--actions') do
          h2(class: 'dashboard__section-title') { 'Quick Actions' }
          div(class: 'dashboard__actions-list') do
            action_links.each do |label, url|
              a(href: url, class: button_classes) { label }
            end
          end
        end
      end

      private

      def action_links
        return [['Add Medicine', '#']] unless url_helpers

        [
          ['Add Medicine', url_helpers.new_medicine_path],
          ['Add Person', url_helpers.new_person_path]
        ]
      end

      def button_classes
        [
          'whitespace-nowrap inline-flex items-center justify-center rounded-md font-medium transition-colors',
          'disabled:pointer-events-none disabled:opacity-50',
          'focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring',
          'px-4 py-2 h-9 text-sm',
          'bg-primary text-primary-foreground shadow',
          'hover:bg-primary/90'
        ].join(' ')
      end
    end
  end
end

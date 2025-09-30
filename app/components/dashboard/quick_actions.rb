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
              Button(href: url, variant: :primary) { label }
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
    end
  end
end

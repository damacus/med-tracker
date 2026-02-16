# frozen_string_literal: true

module Components
  module Dashboard
    # Renders the dashboard quick actions section
    class QuickActions < Components::Base
      include Phlex::Rails::Helpers::T

      attr_reader :url_helpers

      def initialize(url_helpers: nil)
        @url_helpers = url_helpers
        super()
      end

      def view_template
        section(class: 'dashboard__section dashboard__section--actions') do
          Heading(level: 2, class: 'dashboard__section-title') { t('dashboard.quick_actions.title') }
          div(class: 'dashboard__actions-list') do
            action_links.each do |label, url|
              Link(href: url, variant: :primary) { label }
            end
          end
        end
      end

      private

      def action_links
        return [[t('dashboard.quick_actions.add_medicine'), '#']] unless url_helpers

        [
          [t('dashboard.quick_actions.add_medicine'), url_helpers.new_medicine_path],
          [t('dashboard.quick_actions.add_person'), url_helpers.new_person_path]
        ]
      end
    end
  end
end

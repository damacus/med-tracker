# frozen_string_literal: true

module Components
  module Dashboard
    # Dashboard index view component that renders the main dashboard page
    class IndexView < Components::Base
      attr_reader :people, :active_prescriptions, :upcoming_prescriptions, :url_helpers

      def initialize(people:, active_prescriptions:, upcoming_prescriptions:, url_helpers: nil)
        @people = people
        @active_prescriptions = active_prescriptions
        @upcoming_prescriptions = upcoming_prescriptions
        @url_helpers = url_helpers
        super()
      end

      def view_template
        div(class: 'dashboard', data: { testid: 'dashboard' }) do
          render_header
          render Components::Dashboard::QuickStats.new(people:, active_prescriptions:)
          render Components::Dashboard::QuickActions.new(url_helpers:)
          render Components::Dashboard::Schedule.new(
            people:,
            upcoming_prescriptions:,
            url_helpers:
          )
        end
      end

      private

      def render_header
        div(class: 'page-header') do
          h1(class: 'page-title') { 'Medicine Tracker Dashboard' }
        end
      end
    end
  end
end

# frozen_string_literal: true

module Components
  module Dashboard
    # Dashboard index view component that renders the main dashboard page
    class IndexView < Components::Base
      attr_reader :users, :active_prescriptions, :upcoming_prescriptions, :url_helpers

      def initialize(users:, active_prescriptions:, upcoming_prescriptions:, url_helpers: nil)
        @users = users
        @active_prescriptions = active_prescriptions
        @upcoming_prescriptions = upcoming_prescriptions
        @url_helpers = url_helpers
        super()
      end

      def view_template
        div(class: 'dashboard') do
          render_header
          render_quick_stats
          render_quick_actions
          render_medication_schedule
        end
      end

      private

      def render_header
        div(class: 'page-header') do
          h1(class: 'page-title') { 'Medicine Tracker Dashboard' }
        end
      end

      def render_quick_stats
        div(class: 'dashboard__section') do
          h2(class: 'dashboard__section-title') { 'Quick Stats' }
          div(class: 'quick-stats') do
            render_stat_card(users.count, 'Users')
            render_stat_card(active_prescriptions.count, 'Active Prescriptions')
          end
        end
      end

      def render_stat_card(number, label)
        div(class: 'stat-card') do
          div(class: 'stat-card__number') { number }
          div(class: 'stat-card__label') { label }
        end
      end

      def render_quick_actions
        div(class: 'dashboard__section') do
          h2(class: 'dashboard__section-title') { 'Quick Actions' }
          div(class: 'quick-actions') do
            if url_helpers
              link_to(url_helpers.new_medicine_path, class: 'quick-action__button') do
                'Add Medicine'
              end
            else
              a(class: 'quick-action__button', href: '#') do
                'Add Medicine'
              end
            end
          end
        end
      end

      def render_medication_schedule
        div(class: 'dashboard__section') do
          h2(class: 'dashboard__section-title') { 'Medication Schedule By Person' }
          div(class: 'schedule-section') do
            if users.any?
              if upcoming_prescriptions.any?
                render_schedule_content
              else
                render_empty_state('No active prescriptions found', 'Add prescriptions to see them here')
              end
            else
              render_empty_state('No users found', 'Add users to start tracking their medications')
            end
          end
        end
      end

      def render_schedule_content
        div(class: 'schedule-content') do
          upcoming_prescriptions.each do |user, prescriptions|
            render Components::Dashboard::PersonSchedule.new(
              user: user,
              prescriptions: prescriptions,
              take_medicine_url_generator: url_helpers ? lambda { |prescription|
                url_helpers.prescription_take_medicines_path(prescription)
              } : nil
            )
          end
        end
      end

      def render_empty_state(message, help_text)
        div(class: 'empty-state') do
          p(class: 'empty-state__message') { message }
          p(class: 'empty-state__help') { help_text }
        end
      end
    end
  end
end

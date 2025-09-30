# frozen_string_literal: true

module Components
  module Layouts
    # Navigation component that renders differently based on authentication state
    class Navigation < Components::Base
      # Include additional helpers needed for this component
      include Phlex::Rails::Helpers::ButtonTo
      # Initialize with optional current_user parameter (useful for testing)
      # @param [User, nil] current_user - The current user or nil if not authenticated
      def initialize(current_user: nil)
        @current_user = current_user
        super() # Initialize parent class
      end

      # Main template method that renders the navigation bar
      # rubocop:disable Metrics/MethodLength
      def view_template
        nav(class: 'nav') do
          div(class: 'nav__container') do
            # Left side with brand and navigation links
            div(class: 'nav__left') do
              render_brand

              # Only render navigation menu if user is authenticated
              render_navigation_menu if authenticated?
            end

            # Right side with authentication actions
            div(class: 'nav__right') do
              render_auth_actions
            end
          end
        end
      end
      # rubocop:enable Metrics/MethodLength

      private

      # Check if user is authenticated
      # @return [Boolean] true if user is authenticated, false otherwise
      def authenticated?
        # Use provided user if available, otherwise use Current.user
        user = if @current_user.nil?
                 Current.user
               else
                 @current_user
               end

        # Ensure we have a valid user object
        user.present? && !user.nil?
      end

      # Render the brand/logo section
      def render_brand
        div(class: 'nav__brand') do
          link_to('MedTracker', root_path, class: 'nav__brand-link')
        end
      end

      # Render navigation menu with links (only for authenticated users)
      def render_navigation_menu
        div(class: 'nav__menu') do
          link_to('Medicines', medicines_path, class: 'nav__link')
          link_to('People', people_path, class: 'nav__link')
          link_to('Medicine Finder', medicine_finder_path, class: 'nav__link')
        end
      end

      # Render authentication actions (login or sign out)
      def render_auth_actions
        div(class: 'nav__user-menu') do
          if authenticated?
            # Show sign out button for authenticated users
            button_to('Sign out',
                      session_path,
                      method: :delete,
                      class: 'btn btn--secondary nav__button')
          else
            # Show login link for unauthenticated users
            link_to('Login', login_path, class: 'nav__link')
          end
        end
      end
    end
  end
end

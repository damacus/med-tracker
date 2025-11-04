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

      # Render authentication actions (login or profile menu)
      def render_auth_actions
        div(class: 'nav__user-menu') do
          if authenticated?
            render_profile_menu
          else
            # Show login link for unauthenticated users
            link_to('Login', login_path, class: 'nav__link')
          end
        end
      end

      # Render profile dropdown menu for authenticated users
      def render_profile_menu
        render RubyUI::DropdownMenu.new do
          render RubyUI::DropdownMenuTrigger.new(class: 'w-full') do
            render RubyUI::Button.new(variant: :outline) { current_user_name }
          end
          render RubyUI::DropdownMenuContent.new do
            render(RubyUI::DropdownMenuLabel.new { 'My Account' })
            render RubyUI::DropdownMenuSeparator.new
            render RubyUI::DropdownMenuItem.new(href: root_path) { 'Dashboard' }
            render RubyUI::DropdownMenuItem.new(href: '#') { 'Profile' }
            render_admin_menu_item if user_is_admin?
            render RubyUI::DropdownMenuSeparator.new
            render_logout_menu_item
          end
        end
      end

      # Render admin menu item if user is administrator
      def render_admin_menu_item
        render RubyUI::DropdownMenuItem.new(href: admin_root_path) { 'Administration' }
      end

      # Render logout menu item with form submission
      def render_logout_menu_item
        render RubyUI::DropdownMenuItem.new do
          button_to('Logout',
                    session_path,
                    method: :delete,
                    class: 'w-full text-left')
        end
      end

      # Get current user name for display
      def current_user_name
        user = @current_user || Current.user
        user&.name || 'Account'
      end

      # Check if current user is an administrator
      def user_is_admin?
        user = @current_user || Current.user
        user&.administrator? || false
      end
    end
  end
end

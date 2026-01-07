# frozen_string_literal: true

module Components
  module Layouts
    # Navigation component that renders differently based on authentication state
    class Navigation < Components::Base
      # Include additional helpers needed for this component
      include Phlex::Rails::Helpers::ButtonTo
      include Phlex::Rails::Helpers::LinkTo

      # Initialize with optional current_user parameter (useful for testing)
      # @param [User, nil] current_user - The current user or nil if not authenticated
      def initialize(current_user: nil)
        @current_user = current_user
        super() # Initialize parent class
      end

      # Main template method that renders the navigation bar
      def view_template
        header(class: 'sticky top-0 z-40 w-full border-b bg-background/95 backdrop-blur ' \
                      'supports-[backdrop-filter]:bg-background/60') do
          nav(class: 'nav') do
            div(class: 'nav__container flex h-16 items-center justify-between px-4') do
              # Left side with brand and desktop navigation links
              div(class: 'nav__left flex items-center gap-6') do
                render_mobile_menu if authenticated?
                render_brand
                render_desktop_navigation_menu if authenticated?
              end

              # Right side with authentication actions (hidden on mobile if authenticated)
              div(class: 'nav__right flex items-center gap-4') do
                render_auth_actions
              end
            end
          end
        end

        render_bottom_navigation if authenticated?
      end

      private

      # Check if user is authenticated
      # @return [Boolean] true if user is authenticated, false otherwise
      def authenticated?
        # Always check the current authentication state via view context
        # Don't rely on cached @current_user prop for authentication state
        view_context.current_user.present?
      end

      # Render the brand/logo section
      def render_brand
        div(class: 'nav__brand') do
          link_to('MedTracker', root_path, class: 'nav__brand-link')
        end
      end

      # Render the mobile hamburger menu using RubyUI::Sheet
      def render_mobile_menu
        div(class: 'md:hidden') do
          render RubyUI::Sheet.new do
            render RubyUI::SheetTrigger.new do
              render RubyUI::Button.new(variant: :ghost, size: :icon, aria: { label: 'Open menu' }) do
                render Icons::Menu.new(class: 'h-6 w-6')
              end
            end

            render RubyUI::SheetContent.new(side: :left, class: 'w-[300px] sm:w-[400px]') do
              render RubyUI::SheetHeader.new do
                render(RubyUI::SheetTitle.new { 'MedTracker' })
              end

              div(class: 'grid gap-4 py-4') do
                render_mobile_navigation_links
              end

              render RubyUI::SheetFooter.new(class: 'mt-auto') do
                render_mobile_auth_actions
              end
            end
          end
        end
      end

      def render_mobile_navigation_links
        div(class: 'flex flex-col gap-2') do
          link_to(medicines_path, class: 'flex items-center gap-2 px-2 py-1 text-lg font-semibold') do
            render Icons::Pill.new(class: 'h-5 w-5')
            plain 'Medicines'
          end
          link_to(people_path, class: 'flex items-center gap-2 px-2 py-1 text-lg font-semibold') do
            render Icons::Users.new(class: 'h-5 w-5')
            plain 'People'
          end
          link_to(medicine_finder_path, class: 'flex items-center gap-2 px-2 py-1 text-lg font-semibold') do
            render Icons::Search.new(class: 'h-5 w-5')
            plain 'Medicine Finder'
          end
        end
      end

      def render_mobile_auth_actions
        div(class: 'flex flex-col gap-2 w-full') do
          link_to(profile_path, class: 'flex items-center gap-2 px-2 py-1 text-lg font-semibold') do
            render Icons::User.new(class: 'h-5 w-5')
            plain 'Profile'
          end
          if user_is_admin?
            link_to(admin_root_path, class: 'flex items-center gap-2 px-2 py-1 text-lg font-semibold') do
              render Icons::Settings.new(class: 'h-5 w-5')
              plain 'Administration'
            end
          end
          button_to('/logout', method: :post,
                               class: 'flex items-center gap-2 px-2 py-1 text-lg font-semibold text-destructive ' \
                                      'w-full text-left') do
            render Icons::LogOut.new(class: 'h-5 w-5')
            plain 'Logout'
          end
        end
      end

      # Render navigation menu for desktop
      def render_desktop_navigation_menu
        div(class: 'hidden md:flex items-center gap-6') do
          link_to('Medicines', medicines_path,
                  class: 'nav__link text-sm font-medium transition-colors hover:text-primary')
          link_to('People', people_path, class: 'nav__link text-sm font-medium transition-colors hover:text-primary')
          link_to('Medicine Finder', medicine_finder_path,
                  class: 'nav__link text-sm font-medium transition-colors hover:text-primary')
        end
      end

      # Render bottom navigation bar for mobile
      def render_bottom_navigation
        nav(class: 'mobile-nav fixed bottom-0 left-0 z-50 w-full h-16 bg-background border-t md:hidden ' \
                   'flex items-center justify-around px-4 pb-safe') do
          link_to(root_path, class: 'flex flex-col items-center gap-1 text-muted-foreground hover:text-primary') do
            render Icons::Home.new(class: 'h-5 w-5')
            span(class: 'text-[10px] font-medium') { 'Home' }
          end
          link_to(medicines_path, class: 'flex flex-col items-center gap-1 text-muted-foreground hover:text-primary') do
            render Icons::Pill.new(class: 'h-5 w-5')
            span(class: 'text-[10px] font-medium') { 'Medicines' }
          end
          link_to(people_path, class: 'flex flex-col items-center gap-1 text-muted-foreground hover:text-primary') do
            render Icons::Users.new(class: 'h-5 w-5')
            span(class: 'text-[10px] font-medium') { 'People' }
          end
          link_to(profile_path, class: 'flex flex-col items-center gap-1 text-muted-foreground hover:text-primary') do
            render Icons::User.new(class: 'h-5 w-5')
            span(class: 'text-[10px] font-medium') { 'Profile' }
          end
        end
      end

      # Render authentication actions (login or profile menu)
      def render_auth_actions
        div(class: 'nav__user-menu hidden md:block') do
          if authenticated?
            render_profile_menu
          else
            link_to('Login', '/login', class: 'nav__link text-sm font-medium')
          end
        end

        # Mobile login link
        return if authenticated?

        div(class: 'md:hidden') do
          link_to('Login', '/login', class: 'nav__link text-sm font-medium')
        end
      end

      # Render profile dropdown menu for authenticated users
      def render_profile_menu
        render RubyUI::DropdownMenu.new do
          render RubyUI::DropdownMenuTrigger.new(class: 'w-full') do
            Button(variant: :outline) { current_user_name }
          end
          render RubyUI::DropdownMenuContent.new do
            render(RubyUI::DropdownMenuLabel.new { 'My Account' })
            render RubyUI::DropdownMenuSeparator.new
            render RubyUI::DropdownMenuItem.new(href: root_path) { 'Dashboard' }
            render RubyUI::DropdownMenuItem.new(href: profile_path) { 'Profile' }
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

      # Render logout menu item with link using Turbo method
      def render_logout_menu_item
        render RubyUI::DropdownMenuItem.new(
          href: '/logout',
          data: { turbo_method: :post }
        ) { 'Logout' }
      end

      # Get current user name for display
      def current_user_name
        user = @current_user
        user&.name || 'Account'
      end

      # Check if current user is an administrator
      def user_is_admin?
        user = @current_user
        user&.administrator? || false
      end
    end
  end
end

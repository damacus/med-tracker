# frozen_string_literal: true

module Components
  module Layouts
    # Navigation component that renders differently based on authentication state
    class Navigation < Components::Base
      include Phlex::Rails::Helpers::LinkTo
      include Phlex::Rails::Helpers::T

      def initialize(current_user: nil)
        @current_user = current_user
        super()
      end

      def view_template
        header(class: 'sticky top-0 z-40 w-full border-b bg-background/95 backdrop-blur ' \
                      'supports-[backdrop-filter]:bg-background/60') do
          render_skip_link
          nav(class: 'nav') do
            div(class: 'nav__container flex h-16 items-center justify-between px-4') do
              render_left_section
              render_right_section
            end
          end
        end

        render Components::Layouts::BottomNav.new if authenticated?
      end

      private

      def render_skip_link
        a(
          href: '#main-content',
          class: 'sr-only focus:not-sr-only focus:absolute focus:z-50 focus:p-4 ' \
                 'focus:bg-background focus:text-foreground focus:ring-2 focus:ring-ring focus:rounded-md'
        ) { t('layouts.navigation.skip_to_content') }
      end

      def authenticated?
        view_context.current_user.present?
      end

      def render_left_section
        div(class: 'nav__left flex items-center gap-6') do
          render Components::Layouts::MobileMenu.new(current_user: @current_user) if authenticated?
          render_brand
          render Components::Layouts::DesktopNav.new if authenticated?
        end
      end

      def render_right_section
        div(class: 'nav__right flex items-center gap-4') do
          render_auth_actions
        end
      end

      def render_brand
        div(class: 'nav__brand') do
          link_to(t('layouts.navigation.brand'), root_path, class: 'nav__brand-link')
        end
      end

      def render_auth_actions
        div(class: 'nav__user-menu hidden md:block') do
          if authenticated?
            render Components::Layouts::ProfileMenu.new(current_user: @current_user)
          else
            link_to(t('layouts.navigation.login'), '/login', class: 'nav__link text-sm font-medium')
          end
        end

        return if authenticated?

        div(class: 'md:hidden') do
          link_to(t('layouts.navigation.login'), '/login', class: 'nav__link text-sm font-medium')
        end
      end
    end
  end
end

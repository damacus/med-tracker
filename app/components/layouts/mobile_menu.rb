# frozen_string_literal: true

module Components
  module Layouts
    # Mobile hamburger menu using RubyUI::Sheet
    class MobileMenu < Components::Base
      include Phlex::Rails::Helpers::LinkTo
      include Phlex::Rails::Helpers::T

      attr_reader :current_user

      def initialize(current_user: nil)
        @current_user = current_user
        super()
      end

      def view_template
        div(class: 'md:hidden') do
          render RubyUI::Sheet.new do
            render RubyUI::SheetTrigger.new do
              button(
                type: 'button',
                class: 'hamburger hamburger--spring',
                aria: { label: t('layouts.mobile_menu.open_menu'), expanded: 'false' }
              ) do
                span(class: 'hamburger-box') do
                  span(class: 'hamburger-inner')
                end
              end
            end

            render RubyUI::SheetContent.new(side: :left) do
              render RubyUI::SheetHeader.new(class: 'flex flex-row items-center justify-between') do
                render(RubyUI::SheetTitle.new { t('layouts.mobile_menu.brand') })
                close_drawer_button
              end

              div(class: 'py-4') do
                render_navigation_links
              end

              render RubyUI::SheetFooter.new(class: 'mt-auto') do
                render_auth_actions
              end
            end
          end
        end
      end

      private

      def close_drawer_button
        button(
          type: 'button',
          class: 'rounded-sm p-2 opacity-70 ring-offset-background transition-opacity ' \
                 'hover:opacity-100 focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 ' \
                 'min-h-[44px] min-w-[44px] flex items-center justify-center',
          data_action: 'click->ruby-ui--sheet-content#close',
          aria: { label: t('layouts.mobile_menu.close_menu') }
        ) do
          svg(
            width: '15',
            height: '15',
            viewbox: '0 0 15 15',
            fill: 'none',
            xmlns: 'http://www.w3.org/2000/svg',
            class: 'h-4 w-4',
            aria_hidden: 'true'
          ) do |s|
            s.path(
              d: 'M11.7816 4.03157C12.0062 3.80702 12.0062 3.44295 11.7816 3.2184C11.5571 2.99385 ' \
                 '11.193 2.99385 10.9685 3.2184L7.50005 6.68682L4.03164 3.2184C3.80708 2.99385 ' \
                 '3.44301 2.99385 3.21846 3.2184C2.99391 3.44295 2.99391 3.80702 3.21846 4.03157L6.68688 ' \
                 '7.49999L3.21846 10.9684C2.99391 11.193 2.99391 11.557 3.21846 11.7816C3.44301 12.0061 ' \
                 '3.80708 12.0061 4.03164 11.7816L7.50005 8.31316L10.9685 11.7816C11.193 12.0061 11.5571 ' \
                 '12.0061 11.7816 11.7816C12.0062 11.557 12.0062 11.193 11.7816 10.9684L8.31322 ' \
                 '7.49999L11.7816 4.03157Z',
              fill: 'currentColor',
              fill_rule: 'evenodd',
              clip_rule: 'evenodd'
            )
          end
          span(class: 'sr-only') { t('layouts.mobile_menu.close_menu') }
        end
      end

      def render_navigation_links
        div(class: 'flex flex-col gap-2') do
          render RubyUI::Link.new(href: medicines_path, variant: :ghost, size: :xl,
                                  class: 'justify-start gap-4 px-4') do
            render Icons::Pill.new(class: 'h-6 w-6')
            plain t('layouts.mobile_menu.medicines')
          end
          render RubyUI::Link.new(href: people_path, variant: :ghost, size: :xl, class: 'justify-start gap-4 px-4') do
            render Icons::Users.new(class: 'h-6 w-6')
            plain t('layouts.mobile_menu.people')
          end
          render RubyUI::Link.new(href: medicine_finder_path, variant: :ghost, size: :xl,
                                  class: 'justify-start gap-4 px-4') do
            render Icons::Search.new(class: 'h-6 w-6')
            plain t('layouts.mobile_menu.medicine_finder')
          end
        end
      end

      def render_auth_actions
        div(class: 'flex flex-col gap-2 w-full') do
          render RubyUI::Link.new(href: profile_path, variant: :ghost, size: :xl, class: 'justify-start gap-4 px-4') do
            render Icons::User.new(class: 'h-6 w-6')
            plain t('layouts.mobile_menu.profile')
          end
          render_admin_link if user_is_admin?
          render_logout_button
        end
      end

      def render_admin_link
        render RubyUI::Link.new(href: admin_root_path, variant: :ghost, size: :xl,
                                class: 'justify-start gap-4 px-4') do
          render Icons::Settings.new(class: 'h-6 w-6')
          plain t('layouts.mobile_menu.administration')
        end
      end

      def render_logout_button
        form(action: '/logout', method: 'post') do
          input(type: 'hidden', name: 'authenticity_token', value: view_context.form_authenticity_token)
          render RubyUI::Button.new(
            type: :submit, variant: :ghost, size: :xl,
            class: 'w-full justify-start gap-4 px-4 text-destructive hover:text-destructive'
          ) do
            render Icons::LogOut.new(class: 'h-6 w-6')
            plain t('layouts.mobile_menu.logout')
          end
        end
      end

      def user_is_admin?
        current_user&.administrator? || false
      end
    end
  end
end

# frozen_string_literal: true

module Components
  module Layouts
    # Mobile hamburger menu using RubyUI::Sheet
    class MobileMenu < Components::Base
      include Phlex::Rails::Helpers::LinkTo

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
          render Icons::X.new(size: 16)
          span(class: 'sr-only') { t('layouts.mobile_menu.close_menu') }
        end
      end

      def render_navigation_links
        div(class: 'flex flex-col gap-2') do
          render RubyUI::Link.new(href: root_path, variant: :ghost, size: :xl,
                                  class: 'justify-start gap-4 px-4 rounded-2xl') do
            render Icons::Home.new(size: 24)
            plain 'Dashboard'
          end
          render RubyUI::Link.new(href: medicines_path, variant: :ghost, size: :xl,
                                  class: 'justify-start gap-4 px-4 rounded-2xl') do
            render Icons::Pill.new(size: 24)
            plain 'Inventory'
          end
          render RubyUI::Link.new(href: locations_path, variant: :ghost, size: :xl,
                                  class: 'justify-start gap-4 px-4 rounded-2xl') do
            render Icons::Home.new(size: 24)
            plain t('layouts.sidebar.locations')
          end
          render RubyUI::Link.new(href: people_path, variant: :ghost, size: :xl,
                                  class: 'justify-start gap-4 px-4 rounded-2xl') do
            render Icons::Users.new(size: 24)
            plain t('layouts.mobile_menu.people')
          end
          render RubyUI::Link.new(href: medicine_finder_path, variant: :ghost, size: :xl,
                                  class: 'justify-start gap-4 px-4 rounded-2xl') do
            render Icons::Search.new(size: 24)
            plain t('layouts.mobile_menu.medicine_finder')
          end
        end
      end

      def render_auth_actions
        div(class: 'flex flex-col gap-2 w-full') do
          render RubyUI::Link.new(href: profile_path, variant: :ghost, size: :xl,
                                  class: 'justify-start gap-4 px-4 rounded-2xl') do
            render Icons::User.new(size: 24)
            plain t('layouts.mobile_menu.profile')
          end
          render_admin_link if user_is_admin?
          render_logout_button
        end
      end

      def render_admin_link
        render RubyUI::Link.new(href: admin_root_path, variant: :ghost, size: :xl,
                                class: 'justify-start gap-4 px-4 rounded-2xl') do
          render Icons::Settings.new(size: 24)
          plain t('layouts.mobile_menu.administration')
        end
      end

      def render_logout_button
        form(action: '/logout', method: 'post') do
          input(type: 'hidden', name: 'authenticity_token', value: view_context.form_authenticity_token)
          render RubyUI::Button.new(
            type: :submit, variant: :ghost, size: :xl,
            class: 'w-full justify-start gap-4 px-4 rounded-2xl text-destructive ' \
                   'hover:text-destructive hover:bg-destructive/5'
          ) do
            render Icons::LogOut.new(size: 24)
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

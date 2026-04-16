# frozen_string_literal: true

module Components
  module Layouts
    # Mobile hamburger menu using RubyUI::Sheet
    class MobileMenu < Components::Base
      include Phlex::Rails::Helpers::LinkTo
      include Phlex::Rails::Helpers::CurrentPage

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

            render RubyUI::SheetContent.new(
              side: :left,
              class: 'bg-surface-container-high border-outline-variant/30 w-[85vw] max-w-[320px]'
            ) do
              render RubyUI::SheetHeader.new(class: 'flex flex-row items-center justify-between px-2') do
                m3_heading(variant: :title_large, level: 2, class: 'font-black tracking-tight') { t('layouts.mobile_menu.brand') }
                close_drawer_button
              end

              div(class: 'py-6 px-2 overflow-y-auto') do
                render_navigation_links
              end

              render RubyUI::SheetFooter.new(class: 'mt-auto pt-6 px-2 border-t border-outline-variant/30') do
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
          class: 'flex h-11 w-11 min-h-[44px] min-w-[44px] items-center justify-center rounded-full transition-all ' \
                 'hover:bg-secondary-container hover:text-on-secondary-container ' \
                 'focus:outline-none focus:ring-2 focus:ring-primary',
          data_action: 'click->ruby-ui--sheet-content#close',
          aria: { label: t('layouts.mobile_menu.close_menu') }
        ) do
          render Icons::X.new(size: 22)
          span(class: 'sr-only') { t('layouts.mobile_menu.close_menu') }
        end
      end

      def render_navigation_links
        div(class: 'flex flex-col gap-1') do
          nav_link(root_path, Icons::Home, t('layouts.sidebar.dashboard'))
          nav_link(medications_path, Icons::Pill, t('layouts.sidebar.inventory'))
          nav_link(locations_path, Icons::Home, t('layouts.sidebar.locations'))
          nav_link(people_path, Icons::Users, t('layouts.mobile_menu.people'))
          nav_link(medication_finder_path, Icons::Search, t('layouts.mobile_menu.medication_finder'))
        end
      end

      def nav_link(path, icon_class, label)
        is_active = current_page?(path)
        m3_link(
          href: path,
          variant: is_active ? :tonal : :text,
          size: :lg,
          class: "justify-start gap-4 px-4 py-4 rounded-full font-bold #{is_active ? 'bg-secondary-container text-on-secondary-container' : 'text-on-surface-variant'}"
        ) do
          render icon_class.new(size: 24)
          plain label
        end
      end

      def render_auth_actions
        div(class: 'flex flex-col gap-1 w-full') do
          nav_link(profile_path, Icons::User, t('layouts.mobile_menu.profile'))
          nav_link(admin_root_path, Icons::Settings, t('layouts.mobile_menu.administration')) if user_is_admin?
          render_logout_button
        end
      end

      def render_logout_button
        form(action: '/logout', method: 'post', class: 'w-full') do
          input(type: 'hidden', name: 'authenticity_token', value: view_context.form_authenticity_token)
          m3_button(
            type: :submit,
            variant: :text,
            size: :lg,
            class: 'w-full justify-start gap-4 px-4 py-4 rounded-full text-error hover:bg-error/5 font-bold'
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
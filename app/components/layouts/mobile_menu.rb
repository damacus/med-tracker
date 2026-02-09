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
                aria: { label: 'Open menu', expanded: 'false' }
              ) do
                span(class: 'hamburger-box') do
                  span(class: 'hamburger-inner')
                end
              end
            end

            render RubyUI::SheetContent.new(side: :left) do
              render RubyUI::SheetHeader.new do
                render(RubyUI::SheetTitle.new { 'MedTracker' })
              end

              div(class: 'grid gap-4 py-4') do
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

      def render_navigation_links
        div(class: 'flex flex-col gap-2') do
          render RubyUI::Link.new(href: medicines_path, variant: :ghost, size: :xl,
                                  class: 'justify-start gap-4 px-4') do
            render Icons::Pill.new(class: 'h-6 w-6')
            plain 'Medicines'
          end
          render RubyUI::Link.new(href: people_path, variant: :ghost, size: :xl, class: 'justify-start gap-4 px-4') do
            render Icons::Users.new(class: 'h-6 w-6')
            plain 'People'
          end
          render RubyUI::Link.new(href: medicine_finder_path, variant: :ghost, size: :xl,
                                  class: 'justify-start gap-4 px-4') do
            render Icons::Search.new(class: 'h-6 w-6')
            plain 'Medicine Finder'
          end
        end
      end

      def render_auth_actions
        div(class: 'flex flex-col gap-2 w-full') do
          render RubyUI::Link.new(href: profile_path, variant: :ghost, size: :xl, class: 'justify-start gap-4 px-4') do
            render Icons::User.new(class: 'h-6 w-6')
            plain 'Profile'
          end
          render_admin_link if user_is_admin?
          render_logout_button
        end
      end

      def render_admin_link
        render RubyUI::Link.new(href: admin_root_path, variant: :ghost, size: :xl,
                                class: 'justify-start gap-4 px-4') do
          render Icons::Settings.new(class: 'h-6 w-6')
          plain 'Administration'
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
            plain 'Logout'
          end
        end
      end

      def user_is_admin?
        current_user&.administrator? || false
      end
    end
  end
end

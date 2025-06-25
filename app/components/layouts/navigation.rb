# frozen_string_literal: true

module Components
  module Layouts
    class Navigation < Components::Base
      # Include form helpers for the sign out button
      include Phlex::Rails::Helpers::FormWith
      include Phlex::Rails::Helpers::LinkTo

      # Initialize without parameters - we'll use Current.user
      # This follows TDD principles by making the minimal change needed
      def initialize(current_user: nil)
        # Use the passed current_user parameter if provided (useful for testing)
        # or fall back to Current.user from the application's authentication system
        @current_user = current_user || Current.user
      end

      def view_template
        # Add the same class as the original navigation for consistency
        nav(class: "nav") do
          div(class: "nav__container") do
            # Left side with brand and main navigation
            div(class: "nav__left") do
              # Brand/Logo linking to home
              render_brand

              # Navigation menu
              div(class: "nav__menu") do
                link_to("Medicines", medicines_path, class: "nav__link")
                link_to("People", people_path, class: "nav__link")
                link_to("Medicine Finder", medicine_finder_path, class: "nav__link")
              end
            end

            # Right side with authentication controls
            div(class: "nav__right") do
              # Conditionally show login/logout based on authentication state
              if @current_user
                render_sign_out_button
              else
                render_login_link
              end
            end
          end
        end
      end

      private

      def render_brand
        a(href: root_path, class: "nav__brand") do
          svg(class: "nav__logo", xmlns: "http://www.w3.org/2000/svg", fill: "none", viewbox: "0 0 24 24", stroke: "currentColor", width: "24", height: "24") do
            # path(stroke_linecap: "round", stroke_linejoin: "round", stroke_width: "2", d: "M19.428 15.428a2 2 0 00-1.022-.547l-2.387-.477a6 6 0 00-3.86.517l-.318.158a6 6 0 01-3.86.517L6.05 15.21a2 2 0 00-1.806.547M8 4h8l-1 1v5.172a2 2 0 00.586 1.414l5 5c1.26 1.26.367 3.414-1.415 3.414H4.828c-1.782 0-2.674-2.154-1.414-3.414l5-5A2 2 0 009 10.172V5L8 4z")
          end
          span(class: "nav__brand-text") { "Med Tracker" }
        end
      end

      def render_sign_out_button
        form_with(url: session_path, method: :delete) do |form|
          form.button("Sign out", class: "button")
        end
      end

      def render_login_link
        link_to("Login", login_path, class: "button")
      end
    end
  end
end

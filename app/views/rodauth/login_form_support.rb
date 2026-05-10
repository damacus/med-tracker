# frozen_string_literal: true

module Views
  module Rodauth
    module LoginFormSupport
      private

      def render_form_panel
        div(data_login_panel: "form", class: form_panel_classes) do
          div(id: "login-flash", class: "mb-4") { flash_section } if flash_message.present?
          render_login_form
          render_secondary_sign_in_options
        end
      end

      def render_login_form
        render(
          RubyUI::Form.new(
            action: view_context.rodauth.login_path,
            method: :post,
            class: "space-y-4 md:space-y-5",
            data_turbo: "false"
          )
        ) do
          authenticity_token_field
          render_email_field
          render_password_field
          render_form_options
          render_submit_button
        end
      end

      def render_email_field
        render_login_form_field(
          label: t("sessions.login.email_label"),
          input_attrs: email_input_attrs,
          icon: :user,
          error: view_context.rodauth.field_error("email") || view_context.rodauth.field_error("login")
        )
      end

      def render_password_field
        render_login_form_field(
          label: t("sessions.login.password_label"),
          input_attrs: password_input_attrs,
          icon: :lock,
          error: view_context.rodauth.field_error("password"),
          actions: lambda {
            m3_link(
              href: view_context.rodauth.reset_password_request_path,
              variant: :text,
              size: :sm,
              class: "h-auto p-0 text-xs font-bold"
            ) do
              t("sessions.login.forgot_password")
            end
          }
        )
      end

      def render_login_form_field(label:, input_attrs:, icon:, error: nil, actions: nil)
        div(class: "space-y-2") do
          div(class: "flex items-center justify-between") do
            render(
              RubyUI::FormFieldLabel.new(for: input_attrs[:id], class: "text-sm font-semibold text-on-surface-variant") {
                label
              }
            )
            actions&.call
          end

          div(class: "relative") do
            input_attrs[:class] = "h-12 rounded-lg border-outline-variant bg-surface-container-lowest pr-12 shadow-sm md:h-14 " \
              "focus-visible:ring-2 focus-visible:ring-teal-500/25 #{input_attrs[:class]}"
            m3_input(**input_attrs)
            div(class: "pointer-events-none absolute inset-y-0 right-4 flex items-center text-on-surface-variant") do
              render_login_field_icon(icon)
            end
          end

          p(class: "mt-1 text-sm font-medium text-error") { error } if error.present?
        end
      end

      def render_login_field_icon(icon)
        case icon
        when :user
          render(Components::Icons::User.new(size: 21))
        else
          render(Components::Icons::Lock.new(size: 21))
        end
      end

      def render_form_options
        div(class: "flex items-center justify-between px-1 pt-1") do
          div(class: "flex items-center gap-2") do
            input(
              type: "checkbox",
              name: "remember",
              id: "remember",
              value: "t",
              class: "h-5 w-5 rounded border-outline-variant bg-surface-container-lowest text-teal-600 focus:ring-teal-500"
            )
            label(for: "remember", class: "text-sm text-on-surface-variant font-medium") do
              t("sessions.login.remember_me")
            end
          end
        end
      end

      def render_submit_button
        m3_button(
          type: :submit,
          variant: :filled,
          size: :lg,
          class: "w-full rounded-lg bg-teal-600 py-5 font-bold shadow-lg shadow-teal-700/15 hover:bg-teal-700 md:py-6 dark:bg-teal-500 dark:text-slate-950 dark:hover:bg-teal-400"
        ) do
          span(class: "flex w-full items-center justify-center") do
            span(class: "flex-1") { t("sessions.login.submit") }
            render(
              Components::Icons::ChevronRight.new(
                size: 22,
                path: "M9 5L16 12L9 19",
                stroke_width: "2.5"
              )
            )
          end
        end
      end
    end
  end
end

# frozen_string_literal: true

module Views
  module Rodauth
    class UnlockAccountRequest < Views::Rodauth::Base
      include Phlex::Rails::Helpers::FormWith
      include Phlex::Rails::Helpers::LinkTo

      def view_template
        page_layout do
          render_auth_card(
            title: rodauth.unlock_account_request_button,
            subtitle: t('rodauth.unlock_account.instruction')
          ) do
            flash_section
            render_explanatory_text
            render_unlock_request_form
            render_other_options
          end
        end
      end

      private

      def flash_section
        return if flash_message.blank?

        div(id: 'unlock-account-request-flash') do
          render_m3_alert(flash_message, variant: flash_variant)
        end
      end

      def flash_message
        view_context.flash[:alert] || view_context.flash[:notice]
      end

      def flash_variant
        view_context.flash[:alert].present? ? :destructive : :success
      end

      def render_explanatory_text
        div(class: 'text-sm leading-6 text-on-surface-variant font-medium') do
          safe(rodauth.unlock_account_request_explanatory_text)
        end
      end

      def render_unlock_request_form
        render RubyUI::Form.new(
          action: rodauth.unlock_account_request_path, method: :post,
          class: 'space-y-6', data_turbo: 'false'
        ) do
          additional_tags = rodauth.unlock_account_request_additional_form_tags
          safe(additional_tags) if additional_tags.present?
          authenticity_token_field
          render_email_field
          render_submit_button
        end
      end

      def render_email_field
        render_m3_form_field(
          label: t('sessions.login.email_label'),
          input_attrs: {
            type: :email,
            name: rodauth.login_param,
            id: 'email',
            required: true,
            autofocus: true,
            autocomplete: 'email',
            value: view_context.params[rodauth.login_param],
            placeholder: t('sessions.login.email_placeholder')
          }
        )
      end

      def render_submit_button
        render_m3_submit_button(rodauth.unlock_account_request_button)
      end

      def render_other_options
        div(class: 'space-y-4 border-t border-outline-variant/30 pt-8') do
          h3(class: 'text-[10px] font-black uppercase tracking-widest text-on-surface-variant') { t('rodauth.views.reset_password_request.other_options') }
          div(class: 'flex flex-col gap-2 items-start') do
            m3_link(href: rodauth.login_path, variant: :text, size: :sm, class: 'font-bold h-auto p-0 underline') { t('rodauth.views.reset_password_request.back_to_login') }
            m3_link(href: rodauth.reset_password_request_path, variant: :text, size: :sm, class: 'font-bold h-auto p-0 underline') do
              t('sessions.login.forgot_password')
            end
          end
        end
      end
    end
  end
end
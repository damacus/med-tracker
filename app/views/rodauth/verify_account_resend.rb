# frozen_string_literal: true

module Views
  module Rodauth
    class VerifyAccountResend < Views::Rodauth::Base
      include Phlex::Rails::Helpers::FormWith
      include Phlex::Rails::Helpers::LinkTo

      def view_template
        page_layout do
          render_page_header(
            title: t('app.name'),
            subtitle: t('rodauth.views.verify_account_resend.page_subtitle')
          )
          form_section
        end
      end

      private

      def form_section
        render_auth_card(
          title: t('rodauth.views.verify_account_resend.card_title'),
          subtitle: t('rodauth.views.verify_account_resend.card_description')
        ) do
          flash_section
          render_resend_form
          render_other_options
        end
      end

      def render_resend_form
        render RubyUI::Form.new(action: view_context.rodauth.verify_account_resend_path, method: :post, class: 'space-y-6', data_turbo: 'false') do
          authenticity_token_field
          email_field
          submit_button
        end
      end

      def email_field
        render_m3_form_field(
          label: t('rodauth.views.verify_account_resend.email_label'),
          input_attrs: {
            type: :email,
            name: 'email',
            id: 'email',
            required: true,
            autofocus: true,
            autocomplete: 'email',
            placeholder: t('rodauth.views.verify_account_resend.email_placeholder'),
            value: view_context.params[:email]
          }
        )
      end

      def submit_button
        render_m3_submit_button(t('rodauth.views.verify_account_resend.submit'))
      end

      def render_other_options
        div(class: 'space-y-4 border-t border-outline-variant/30 pt-8') do
          m3_text(variant: :body_medium, class: 'text-on-surface-variant font-medium') do
            plain "#{t('rodauth.views.verify_account_resend.back_to_login')} "
            m3_link(href: view_context.rodauth.login_path, variant: :text, class: 'p-0 h-auto font-black underline') do
              t('sessions.login.heading')
            end
          end
        end
      end

      def flash_section
        return if flash_message.blank?

        render_m3_alert(flash_message, variant: flash_variant)
      end

      def flash_message
        view_context.flash[:alert] || view_context.flash[:notice]
      end

      def flash_variant
        view_context.flash[:alert].present? ? :destructive : :success
      end
    end
  end
end
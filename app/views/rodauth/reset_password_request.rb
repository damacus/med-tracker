# frozen_string_literal: true

module Views
  module Rodauth
    class ResetPasswordRequest < Views::Rodauth::Base
      include Phlex::Rails::Helpers::FormWith
      include Phlex::Rails::Helpers::LinkTo

      def view_template
        page_layout do
          render_page_header(
            title: t('app.name'),
            subtitle: t('rodauth.views.reset_password_request.page_subtitle')
          )
          form_section
        end
      end

      private

      def flash_section
        return if flash_message.blank?

        div(id: 'reset-flash') do
          render RubyUI::Alert.new(variant: flash_variant) do
            plain(flash_message)
          end
        end
      end

      def flash_message
        view_context.flash[:alert] || view_context.flash[:notice]
      end

      def flash_variant
        view_context.flash[:alert].present? ? :destructive : :success
      end

      def form_section
        render_auth_card(
          title: t('rodauth.views.reset_password_request.card_title'),
          subtitle: t('rodauth.views.reset_password_request.card_description')
        ) do
          flash_section
          render_reset_form
          render_other_options
        end
      end

      def render_reset_form
        render RubyUI::Form.new(action: view_context.rodauth.reset_password_request_path, method: :post, class: 'space-y-6', data_turbo: 'false') do
          authenticity_token_field
          email_field
          submit_button
        end
      end

      def email_field
        render_m3_form_field(
          label: t('rodauth.views.reset_password_request.email_label'),
          input_attrs: {
            type: :email,
            name: 'email',
            id: 'email',
            required: true,
            autofocus: true,
            autocomplete: 'email',
            placeholder: t('rodauth.views.reset_password_request.email_placeholder'),
            value: view_context.params[:email]
          }
        )
      end

      def submit_button
        render_m3_submit_button(t('rodauth.views.reset_password_request.submit'))
      end

      def render_other_options
        div(class: 'space-y-4 border-t border-outline-variant/30 pt-8') do
          m3_text(variant: :body_medium, class: 'text-on-surface-variant font-medium') do
            plain "#{t('rodauth.views.reset_password_request.back_to_login')} "
            m3_link(href: view_context.rodauth.login_path, variant: :text, class: 'p-0 h-auto font-black underline') do
              t('sessions.login.heading')
            end
          end
        end
      end
    end
  end
end
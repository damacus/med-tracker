# frozen_string_literal: true

module Views
  module Rodauth
    class VerifyAccount < Views::Rodauth::Base
      include Phlex::Rails::Helpers::FormWith
      include Phlex::Rails::Helpers::LinkTo

      def view_template
        page_layout do
          render_page_header(
            title: t('app.name'),
            subtitle: t('rodauth.views.verify_account.page_subtitle')
          )
          form_section
        end
      end

      private

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

      def form_section
        render_auth_card(
          title: t('rodauth.views.verify_account.card_title'),
          subtitle: t('rodauth.views.verify_account.card_description')
        ) do
          flash_section
          render_verify_form
        end
      end

      def render_verify_form
        render RubyUI::Form.new(action: view_context.rodauth.verify_account_path, method: :post, class: 'space-y-6', data_turbo: 'false') do
          authenticity_token_field
          key_field
          submit_button
        end
      end

      def key_field
        input(type: 'hidden', name: 'key', value: view_context.params[:key])
      end

      def submit_button
        render_m3_submit_button(t('rodauth.views.verify_account.submit'))
      end
    end
  end
end
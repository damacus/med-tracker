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

      def flash_section
        return if flash_message.blank?

        div(id: 'verify-flash') do
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
        render RubyUI::Card.new(class: card_classes) do
          render_card_header
          render_card_content
        end
      end

      def render_card_header
        render RubyUI::CardHeader.new(class: 'space-y-2 bg-white/60') do
          render RubyUI::CardTitle.new(class: 'text-2xl font-semibold text-slate-900') { t('rodauth.views.verify_account_resend.card_title') }
          render RubyUI::CardDescription.new(class: 'text-base text-slate-600') do
            plain t('rodauth.views.verify_account_resend.card_description')
          end
        end
      end

      def render_card_content
        render RubyUI::CardContent.new(class: 'space-y-6 p-6 sm:p-8') do
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
        render RubyUI::FormField.new do
          render RubyUI::FormFieldLabel.new(for: 'email') { t('rodauth.views.verify_account_resend.email_label') }
          render RubyUI::Input.new(
            type: :email,
            name: 'email',
            id: 'email',
            required: true,
            autofocus: true,
            autocomplete: 'email',
            placeholder: t('rodauth.views.verify_account_resend.email_placeholder'),
            value: view_context.params[:email]
          )
        end
      end

      def submit_button
        render RubyUI::Button.new(type: :submit, variant: :primary, size: :md, class: 'w-full') do
          t('rodauth.views.verify_account_resend.submit')
        end
      end

      def render_other_options
        div(class: 'space-y-3 border-t border-slate-200 pt-6') do
          h3(class: 'text-sm font-medium text-slate-700') { t('rodauth.views.verify_account_resend.other_options') }
          div(class: 'flex flex-col gap-2 text-sm') do
            render RubyUI::Link.new(href: view_context.rodauth.login_path, variant: :link) do
              t('rodauth.views.verify_account_resend.back_to_login')
            end
            render RubyUI::Link.new(href: view_context.rodauth.create_account_path, variant: :link) do
              t('rodauth.views.verify_account_resend.create_account')
            end
          end
        end
      end
    end
  end
end

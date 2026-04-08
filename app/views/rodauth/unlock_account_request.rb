# frozen_string_literal: true

module Views
  module Rodauth
    class UnlockAccountRequest < Views::Rodauth::Base
      include Phlex::Rails::Helpers::FormWith
      include Phlex::Rails::Helpers::LinkTo

      def view_template
        page_layout do
          render_page_header(
            title: t('app.name'),
            subtitle: t('rodauth.unlock_account.title')
          )
          form_section
        end
      end

      private

      def form_section
        render RubyUI::Card.new(class: card_classes) do
          render_card_header
          render RubyUI::CardContent.new(class: 'space-y-6 p-6 sm:p-8') do
            flash_section
            explanatory_text
            unlock_request_form
            other_options
          end
        end
      end

      def render_card_header
        render RubyUI::CardHeader.new(class: 'space-y-2 bg-card/60') do
          render RubyUI::CardTitle.new(class: 'text-2xl font-semibold text-foreground') do
            rodauth.unlock_account_request_button
          end
          render RubyUI::CardDescription.new(class: 'text-base text-muted-foreground') do
            plain t('rodauth.unlock_account.instruction')
          end
        end
      end

      def flash_section
        return if flash_message.blank?

        div(id: 'unlock-account-request-flash') do
          render RubyUI::Alert.new(variant: flash_variant) { plain(flash_message) }
        end
      end

      def flash_message
        view_context.flash[:alert] || view_context.flash[:notice]
      end

      def flash_variant
        view_context.flash[:alert].present? ? :destructive : :success
      end

      def explanatory_text
        div(class: 'text-sm leading-6 text-muted-foreground') do
          safe(rodauth.unlock_account_request_explanatory_text)
        end
      end

      def unlock_request_form
        render RubyUI::Form.new(
          action: rodauth.unlock_account_request_path, method: :post,
          class: 'space-y-6', data_turbo: 'false'
        ) do
          additional_tags = rodauth.unlock_account_request_additional_form_tags
          safe(additional_tags) if additional_tags.present?
          authenticity_token_field
          email_field
          submit_button
        end
      end

      def email_field
        render RubyUI::FormField.new do
          render RubyUI::FormFieldLabel.new(for: 'email') { t('sessions.login.email_label') }
          render RubyUI::Input.new(
            type: :email,
            name: rodauth.login_param,
            id: 'email',
            required: true,
            autofocus: true,
            autocomplete: 'email',
            value: view_context.params[rodauth.login_param],
            placeholder: t('sessions.login.email_placeholder')
          )
        end
      end

      def submit_button
        render RubyUI::Button.new(type: :submit, variant: :primary, class: 'w-full') do
          rodauth.unlock_account_request_button
        end
      end

      def other_options
        div(class: 'space-y-3 border-t border-border pt-6') do
          h3(class: 'text-sm font-medium text-foreground') { t('rodauth.views.reset_password_request.other_options') }
          div(class: 'flex flex-col gap-2 text-sm') do
            render RubyUI::Link.new(href: rodauth.login_path, variant: :link) { t('rodauth.views.reset_password_request.back_to_login') }
            render RubyUI::Link.new(href: rodauth.reset_password_request_path, variant: :link) do
              t('sessions.login.forgot_password')
            end
          end
        end
      end
    end
  end
end

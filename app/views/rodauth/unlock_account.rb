# frozen_string_literal: true

module Views
  module Rodauth
    class UnlockAccount < Views::Rodauth::Base
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
          render RubyUI::CardHeader.new(class: 'space-y-2 bg-card/60') do
            render RubyUI::CardTitle.new(class: 'text-2xl font-semibold text-foreground') do
              rodauth.unlock_account_button
            end
            render RubyUI::CardDescription.new(class: 'text-base text-muted-foreground') do
              plain t('rodauth.unlock_account.instruction')
            end
          end

          render RubyUI::CardContent.new(class: 'space-y-6 p-6 sm:p-8') do
            flash_section
            explanatory_text
            unlock_form
          end
        end
      end

      def flash_section
        return if flash_message.blank?

        div(id: 'unlock-account-flash') do
          render RubyUI::Alert.new(variant: :destructive) { plain(flash_message) }
        end
      end

      def flash_message
        view_context.flash[:alert]
      end

      def explanatory_text
        div(class: 'text-sm leading-6 text-muted-foreground') do
          safe(rodauth.unlock_account_explanatory_text)
        end
      end

      def unlock_form
        render RubyUI::Form.new(action: rodauth.unlock_account_path, method: :post, class: 'space-y-6', data_turbo: 'false') do
          additional_tags = rodauth.unlock_account_additional_form_tags
          safe(additional_tags) if additional_tags.present?
          authenticity_token_field
          key_field
          password_field if rodauth.unlock_account_requires_password?
          submit_button
        end
      end

      def key_field
        key = view_context.params[rodauth.unlock_account_key_param]
        input(type: 'hidden', name: rodauth.unlock_account_key_param, value: key) if key.present?
      end

      def password_field
        render RubyUI::FormField.new do
          render RubyUI::FormFieldLabel.new(for: 'password') { t('sessions.login.password_label') }
          render RubyUI::Input.new(
            type: :password,
            name: rodauth.password_param,
            id: 'password',
            required: true,
            autocomplete: 'current-password',
            placeholder: t('sessions.login.password_placeholder')
          )
        end
      end

      def submit_button
        render RubyUI::Button.new(type: :submit, variant: :primary, class: 'w-full') do
          rodauth.unlock_account_button
        end
      end
    end
  end
end

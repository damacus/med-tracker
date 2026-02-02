# frozen_string_literal: true

module Views
  module Rodauth
    class OtpDisable < Views::Rodauth::Base
      def view_template
        page_layout do
          render_page_header(
            title: 'Disable authenticator app',
            subtitle: 'Confirm your password to turn off time-based codes.'
          )
          form_section
        end
      end

      private

      def form_section
        render RubyUI::Card.new(class: card_classes) do
          render RubyUI::CardHeader.new(class: 'space-y-2 bg-white/60') do
            render RubyUI::CardTitle.new(class: 'text-xl font-semibold text-slate-900') do
              'Turn off two-factor codes'
            end
            render RubyUI::CardDescription.new(class: 'text-base text-slate-600') do
              'You can re-enable your authenticator app at any time.'
            end
          end
          render RubyUI::CardContent.new(class: 'space-y-6 p-6 sm:p-8') do
            otp_disable_form
          end
        end
      end

      def otp_disable_form
        render RubyUI::Form.new(action: rodauth.otp_disable_path, method: :post, class: 'space-y-6', data_turbo: 'false') do
          additional_tags = rodauth.otp_disable_additional_form_tags
          safe(additional_tags) if additional_tags.present?
          authenticity_token_field
          password_field if rodauth.two_factor_modifications_require_password?
          submit_button
        end
      end

      def password_field
        render RubyUI::FormField.new do
          render RubyUI::FormFieldLabel.new(for: 'password') { 'Password' }
          render RubyUI::Input.new(
            type: :password,
            name: 'password',
            id: 'password',
            required: true,
            autocomplete: 'current-password',
            placeholder: 'Enter your password'
          )
        end
      end

      def submit_button
        render RubyUI::Button.new(type: :submit, variant: :destructive, size: :md, class: 'w-full') do
          rodauth.otp_disable_button
        end
      end
    end
  end
end

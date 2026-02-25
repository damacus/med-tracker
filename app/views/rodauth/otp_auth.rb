# frozen_string_literal: true

module Views
  module Rodauth
    class OtpAuth < Views::Rodauth::Base
      def view_template
        page_layout do
          render_page_header(
            title: 'Enter your authentication code',
            subtitle: 'Use the code from your authenticator app to continue.'
          )
          form_section
        end
      end

      private

      def form_section
        render RubyUI::Card.new(class: card_classes) do
          render RubyUI::CardHeader.new(class: 'space-y-2 bg-white/60') do
            render RubyUI::CardTitle.new(class: 'text-xl font-semibold text-slate-900') do
              'Authenticator code'
            end
            render RubyUI::CardDescription.new(class: 'text-base text-slate-600') do
              'Enter the 6-digit code to finish signing in.'
            end
          end
          render RubyUI::CardContent.new(class: 'space-y-6 p-6 sm:p-8') do
            otp_form
          end
        end
      end

      def otp_form
        render RubyUI::Form.new(action: rodauth.otp_auth_path, method: :post, class: 'space-y-6', data_turbo: 'false') do
          additional_tags = rodauth.otp_auth_additional_form_tags
          safe(additional_tags) if additional_tags.present?
          authenticity_token_field
          otp_code_field
          submit_button
        end
      end

      def otp_code_field
        render RubyUI::FormField.new do
          render RubyUI::FormFieldLabel.new(for: 'otp-auth-code') { rodauth.otp_auth_label }
          render RubyUI::Input.new(
            type: :text,
            name: rodauth.otp_auth_param,
            id: 'otp-auth-code',
            required: true,
            autocomplete: 'one-time-code',
            inputmode: 'numeric',
            pattern: '[0-9]*',
            maxlength: 6,
            placeholder: 'Enter 6-digit code'
          )
        end
      end

      def submit_button
        render RubyUI::Button.new(type: :submit, variant: :primary, size: :md, class: 'w-full') do
          rodauth.otp_auth_button
        end
      end
    end
  end
end

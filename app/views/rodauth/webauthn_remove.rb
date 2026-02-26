# frozen_string_literal: true

module Views
  module Rodauth
    class WebauthnRemove < Views::Rodauth::Base
      def view_template
        page_layout do
          render_page_header(
            title: t('rodauth.views.webauthn_remove.page_title'),
            subtitle: t('rodauth.views.webauthn_remove.page_subtitle')
          )
          form_section
        end
      end

      private

      def form_section
        render RubyUI::Card.new(class: card_classes) do
          render RubyUI::CardHeader.new(class: 'space-y-2 bg-white/60') do
            render RubyUI::CardTitle.new(class: 'text-xl font-semibold text-slate-900') do
              t('rodauth.views.webauthn_remove.card_title')
            end
            render RubyUI::CardDescription.new(class: 'text-base text-slate-600') do
              t('rodauth.views.webauthn_remove.card_description')
            end
          end
          render RubyUI::CardContent.new(class: 'space-y-6 p-6 sm:p-8') do
            webauthn_remove_form
          end
        end
      end

      def webauthn_remove_form
        render RubyUI::Form.new(action: rodauth.webauthn_remove_path, method: :post, class: 'space-y-6', data_turbo: 'false') do
          render_additional_tags
          authenticity_token_field
          hidden_id_field
          password_field if rodauth.two_factor_modifications_require_password?
          submit_button
        end
      end

      def render_additional_tags
        return unless rodauth.respond_to?(:webauthn_remove_additional_form_tags)

        tags = rodauth.webauthn_remove_additional_form_tags
        safe(tags) if tags.present?
      end

      def hidden_id_field
        input(
          type: 'hidden',
          name: rodauth.webauthn_remove_param,
          value: view_context.params[rodauth.webauthn_remove_param]
        )
      end

      def password_field
        render RubyUI::FormField.new do
          render RubyUI::FormFieldLabel.new(for: 'password') { t('rodauth.views.webauthn_remove.password_label') }
          render RubyUI::Input.new(
            type: :password,
            name: 'password',
            id: 'password',
            required: true,
            autocomplete: 'current-password',
            placeholder: t('rodauth.views.webauthn_remove.password_placeholder')
          )
        end
      end

      def submit_button
        render RubyUI::Button.new(type: :submit, variant: :destructive, size: :md, class: 'w-full') do
          rodauth.webauthn_remove_button
        end
      end
    end
  end
end

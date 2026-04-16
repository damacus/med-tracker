# frozen_string_literal: true

module Views
  module Rodauth
    class WebauthnSetup < Views::Rodauth::Base
      include Phlex::Rails::Helpers::FormWith
      include Phlex::Rails::Helpers::LinkTo

      def view_template
        page_layout do
          render_page_header(
            title: t('rodauth.views.webauthn_setup.page_title'),
            subtitle: t('rodauth.views.webauthn_setup.page_subtitle')
          )
          form_section
        end
      end

      private

      def form_section
        render_auth_card(
          title: t('rodauth.views.webauthn_setup.card_title'),
          subtitle: t('rodauth.views.webauthn_setup.card_description')
        ) do
          render_info_section
          render_setup_form
        end
      end

      def render_info_section
        div(class: 'space-y-3 rounded-2xl border border-outline-variant/30 bg-secondary-container/30 p-5') do
          div(class: 'flex items-start gap-3') do
            render_info_icon
            div(class: 'space-y-1') do
              m3_heading(variant: :title_small, class: 'font-bold text-foreground') { t('rodauth.views.webauthn_setup.info_title') }
              m3_text(variant: :body_small, class: 'text-on-surface-variant font-medium') do
                t('rodauth.views.webauthn_setup.info_description')
              end
            end
          end
        end
      end

      def render_info_icon
        div(class: 'flex h-10 w-10 flex-shrink-0 items-center justify-center rounded-full bg-primary/12') do
          render Icons::Lock.new(size: 20, class: 'text-primary')
        end
      end

      def render_setup_form
        credential = view_context.rodauth.new_webauthn_credential
        div(class: 'border-t border-border pt-6') do
          render_webauthn_form(credential)
          render_webauthn_script
        end
      rescue StandardError => e
        Rails.logger.error("Failed to render WebAuthn setup form: #{e.message}")
        div(class: 'border-t border-border pt-6') do
          p(class: 'text-sm text-destructive') do
            'Unable to initialize passkey registration. Please try again later.'
          end
        end
      end

      def render_webauthn_form(credential_options)
        form(
          method: :post,
          action: view_context.rodauth.webauthn_setup_path,
          id: 'webauthn-setup-form',
          data_credential_options: credential_options.as_json.to_json,
          class: 'space-y-6'
        ) do
          additional_tags = view_context.rodauth.webauthn_setup_additional_form_tags
          safe(additional_tags) if additional_tags.present?
          authenticity_token_field
          hidden_webauthn_fields(credential_options)
          password_field
          submit_button
        end
      end

      def hidden_webauthn_fields(credential_options)
        input(type: 'hidden', id: 'webauthn-setup', name: view_context.rodauth.webauthn_setup_param, value: '')
        input(type: 'hidden', name: view_context.rodauth.webauthn_setup_challenge_param, value: credential_options.challenge)
        input(type: 'hidden', name: view_context.rodauth.webauthn_setup_challenge_hmac_param, value: view_context.rodauth.compute_hmac(credential_options.challenge))
      end

      def password_field
        render_m3_form_field(
          label: t('rodauth.views.webauthn_setup.current_password_label'),
          input_attrs: {
            type: :password,
            name: 'password',
            id: 'password',
            required: true,
            autocomplete: 'current-password',
            placeholder: t('rodauth.views.webauthn_setup.current_password_placeholder')
          }
        ) do
          p(class: 'mt-1 px-1 text-xs text-on-surface-variant font-medium') { t('rodauth.views.webauthn_setup.current_password_hint') }
        end
      end

      def submit_button
        render_m3_submit_button(t('rodauth.views.webauthn_setup.submit'))
      end

      def render_webauthn_script
        script(src: "#{view_context.rodauth.webauthn_js_host}#{view_context.rodauth.webauthn_setup_js_path}", nonce: view_context.content_security_policy_nonce)
      end
    end
  end
end

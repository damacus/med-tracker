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
        render RubyUI::Card.new(class: card_classes) do
          render_card_header
          render_card_content
        end
      end

      def card_classes
        "#{CARD_CLASSES} overflow-hidden"
      end

      def render_card_header
        render RubyUI::CardHeader.new(class: 'space-y-2 bg-white/60') do
          render RubyUI::CardTitle.new(class: 'text-xl font-semibold text-slate-900') do
            t('rodauth.views.webauthn_setup.card_title')
          end
          render RubyUI::CardDescription.new(class: 'text-base text-slate-600') do
            plain t('rodauth.views.webauthn_setup.card_description')
          end
        end
      end

      def render_card_content
        render RubyUI::CardContent.new(class: 'space-y-6 p-6 sm:p-8') do
          render_info_section
          render_setup_form
        end
      end

      def render_info_section
        div(class: 'bg-sky-50 rounded-xl p-4 space-y-3') do
          div(class: 'flex items-start gap-3') do
            render_info_icon
            div(class: 'space-y-1') do
              h4(class: 'font-medium text-slate-900') { t('rodauth.views.webauthn_setup.info_title') }
              p(class: 'text-sm text-slate-600') do
                t('rodauth.views.webauthn_setup.info_description')
              end
            end
          end
        end
      end

      def render_info_icon
        div(class: 'flex-shrink-0 w-10 h-10 bg-sky-100 rounded-full flex items-center justify-center') do
          render Icons::Lock.new(size: 20, class: 'text-sky-600')
        end
      end

      def render_setup_form
        div(class: 'border-t border-slate-200 pt-6') do
          form(
            method: :post,
            action: view_context.rodauth.webauthn_setup_path,
            id: 'webauthn-setup-form',
            class: 'space-y-6'
          ) do
            additional_tags = view_context.rodauth.webauthn_setup_additional_form_tags
            safe(additional_tags) if additional_tags.present?
            authenticity_token_field
            hidden_webauthn_fields
            password_field
            submit_button
          end

          render_webauthn_script
        end
      end

      def hidden_webauthn_fields
        input(type: 'hidden', id: 'webauthn-setup', name: view_context.rodauth.webauthn_setup_param, value: '')
      end

      def password_field
        render RubyUI::FormField.new do
          render RubyUI::FormFieldLabel.new(for: 'password') { t('rodauth.views.webauthn_setup.current_password_label') }
          render RubyUI::Input.new(
            type: :password,
            name: 'password',
            id: 'password',
            required: true,
            autocomplete: 'current-password',
            placeholder: t('rodauth.views.webauthn_setup.current_password_placeholder')
          )
          p(class: 'text-xs text-slate-500 mt-1') { t('rodauth.views.webauthn_setup.current_password_hint') }
        end
      end

      def submit_button
        render RubyUI::Button.new(type: :submit, variant: :primary, size: :md, class: 'w-full') do
          t('rodauth.views.webauthn_setup.submit')
        end
      end

      def render_webauthn_script
        script(src: "#{view_context.rodauth.webauthn_js_host}#{view_context.rodauth.webauthn_setup_js_path}")
      end
    end
  end
end

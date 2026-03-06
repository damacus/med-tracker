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
        render RubyUI::CardHeader.new(class: 'space-y-2 bg-card/60') do
          render RubyUI::CardTitle.new(class: 'text-xl font-semibold text-foreground') do
            t('rodauth.views.webauthn_setup.card_title')
          end
          render RubyUI::CardDescription.new(class: 'text-base text-muted-foreground') do
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
        div(class: 'space-y-3 rounded-xl border border-border bg-muted/60 p-4') do
          div(class: 'flex items-start gap-3') do
            render_info_icon
            div(class: 'space-y-1') do
              h4(class: 'font-medium text-foreground') { t('rodauth.views.webauthn_setup.info_title') }
              p(class: 'text-sm text-muted-foreground') do
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
        div(class: 'border-t border-border pt-6') do
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
          p(class: 'mt-1 text-xs text-muted-foreground') { t('rodauth.views.webauthn_setup.current_password_hint') }
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

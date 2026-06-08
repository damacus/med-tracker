# frozen_string_literal: true

module Views
  module Rodauth
    class RecoveryCodes < Views::Rodauth::Base
      def view_template
        page_layout do
          render_page_header(
            title: t('rodauth.views.recovery_codes.page_title'),
            subtitle: t('rodauth.views.recovery_codes.page_subtitle')
          )
          form_section
        end
      end

      private

      def form_section
        render_auth_card(
          title: t('rodauth.views.recovery_codes.card_title'),
          subtitle: t('rodauth.views.recovery_codes.card_description')
        ) do
          flash_section
          safety_panel
          password_form
        end
      end

      def flash_section
        return if flash_message.blank?

        render_m3_alert(flash_message)
      end

      def flash_message
        view_context.flash[:alert] || view_context.rodauth.field_error(rodauth.password_param)
      end

      def safety_panel
        div(class: 'rounded-2xl border border-outline-variant/40 bg-secondary-container/30 p-5') do
          div(class: 'flex items-start gap-3') do
            div(class: 'flex h-10 w-10 flex-shrink-0 items-center justify-center rounded-full bg-primary/12 text-primary') do
              render Icons::FileText.new(size: 20)
            end
            div(class: 'space-y-1') do
              m3_heading(variant: :title_small, class: 'font-bold text-foreground') do
                t('rodauth.views.recovery_codes.safety_title')
              end
              m3_text(variant: :body_small, class: 'text-on-surface-variant font-medium') do
                t('rodauth.views.recovery_codes.safety_description')
              end
            end
          end
        end
      end

      def password_form
        render RubyUI::Form.new(action: rodauth.recovery_codes_path, method: :post, class: 'space-y-6', data_turbo: 'false') do
          render_additional_tags
          authenticity_token_field
          password_field
          submit_button
        end
      end

      def render_additional_tags
        tags = rodauth.recovery_codes_additional_form_tags
        safe(tags) if tags.present?
      end

      def password_field
        render_m3_form_field(
          label: rodauth.password_label,
          input_attrs: {
            type: :password,
            name: rodauth.password_param,
            id: 'password',
            required: true,
            autocomplete: 'current-password',
            placeholder: t('rodauth.views.recovery_codes.password_placeholder')
          },
          error: view_context.rodauth.field_error(rodauth.password_param)
        )
      end

      def submit_button
        render_m3_submit_button(rodauth.view_recovery_codes_button)
      end
    end
  end
end

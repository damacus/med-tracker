# frozen_string_literal: true

module Views
  module Rodauth
    class RecoveryAuth < Views::Rodauth::Base
      def view_template
        page_layout do
          render_page_header(
            title: t('rodauth.views.two_factor_auth.page_title'),
            subtitle: t('rodauth.views.two_factor_auth.page_subtitle')
          )
          form_section
        end
      end

      private

      def form_section
        render_auth_card(
          title: t('rodauth.views.recovery_auth.card_title'),
          subtitle: t('rodauth.views.recovery_auth.card_description')
        ) do
          flash_section
          recovery_code_panel
          recovery_auth_form
        end
      end

      def flash_section
        return if flash_message.blank?

        render_m3_alert(flash_message)
      end

      def flash_message
        view_context.flash[:alert] || view_context.rodauth.field_error(rodauth.recovery_codes_param)
      end

      def recovery_code_panel
        div(class: 'rounded-2xl border border-outline-variant/40 bg-secondary-container/30 p-5') do
          div(class: 'flex items-start gap-3') do
            div(class: 'flex h-10 w-10 flex-shrink-0 items-center justify-center rounded-full bg-primary/12 text-primary') do
              render Icons::FileText.new(size: 20)
            end
            div(class: 'space-y-1') do
              m3_heading(variant: :title_small, class: 'font-bold text-foreground') do
                t('rodauth.views.recovery_auth.info_title')
              end
              m3_text(variant: :body_small, class: 'text-on-surface-variant font-medium') do
                t('rodauth.views.recovery_auth.info_description')
              end
            end
          end
        end
      end

      def recovery_auth_form
        render RubyUI::Form.new(
          action: rodauth.recovery_auth_path, method: :post,
          class: 'space-y-6', data_turbo: 'false'
        ) do
          additional_tags = rodauth.recovery_auth_additional_form_tags
          safe(additional_tags) if additional_tags.present?
          authenticity_token_field
          recovery_code_field
          submit_button
        end
      end

      def recovery_code_field
        render_m3_form_field(
          label: rodauth.recovery_codes_label,
          input_attrs: {
            type: :text,
            name: rodauth.recovery_codes_param,
            id: 'recovery-code',
            required: true,
            autocomplete: 'one-time-code',
            placeholder: rodauth.recovery_codes_label
          },
          error: view_context.rodauth.field_error(rodauth.recovery_codes_param)
        )
      end

      def submit_button
        render_m3_submit_button(rodauth.recovery_auth_button)
      end
    end
  end
end

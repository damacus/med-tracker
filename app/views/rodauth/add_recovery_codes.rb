# frozen_string_literal: true

module Views
  module Rodauth
    class AddRecoveryCodes < Views::Rodauth::Base
      def view_template
        page_layout do
          render_page_header(
            title: t('rodauth.views.add_recovery_codes.page_title'),
            subtitle: t('rodauth.views.add_recovery_codes.page_subtitle')
          )
          codes_section
        end
      end

      private

      def codes_section
        render_auth_card(
          title: t('rodauth.views.add_recovery_codes.card_title'),
          subtitle: t('rodauth.views.add_recovery_codes.card_description')
        ) do
          flash_section
          codes_notice
          codes_grid
          add_codes_form if rodauth.can_add_recovery_codes?
        end
      end

      def flash_section
        return if flash_message.blank?

        render_m3_alert(flash_message)
      end

      def flash_message
        view_context.flash[:alert] || view_context.rodauth.field_error(rodauth.password_param)
      end

      def codes_notice
        div(class: 'rounded-2xl border border-warning/30 bg-warning-container/40 p-5') do
          div(class: 'flex items-start gap-3') do
            div(class: 'flex h-10 w-10 flex-shrink-0 items-center justify-center rounded-full bg-warning/15 text-warning') do
              render Icons::AlertCircle.new(size: 20)
            end
            div(class: 'space-y-1') do
              m3_heading(variant: :title_small, class: 'font-bold text-foreground') do
                t('rodauth.views.add_recovery_codes.notice_title')
              end
              m3_text(variant: :body_small, class: 'text-on-surface-variant font-medium') do
                t('rodauth.views.add_recovery_codes.notice_description')
              end
            end
          end
        end
      end

      def codes_grid
        div(id: 'recovery-codes', class: 'grid gap-3 sm:grid-cols-2') do
          rodauth.recovery_codes.each do |recovery_code|
            recovery_code_item(recovery_code)
          end
        end
      end

      def recovery_code_item(recovery_code)
        div(class: 'flex items-center gap-3 rounded-2xl border border-outline-variant/50 bg-surface-container-low p-3') do
          code(class: 'min-w-0 flex-1 select-all truncate font-mono text-sm font-bold tracking-wide text-foreground') do
            plain recovery_code
          end
          copy_button(recovery_code)
        end
      end

      def copy_button(recovery_code)
        button(
          type: 'button',
          class: 'flex h-10 w-10 flex-shrink-0 items-center justify-center rounded-xl bg-surface-container-highest text-primary transition-all hover:scale-105 active:scale-95',
          title: t('rodauth.views.add_recovery_codes.copy_to_clipboard'),
          data: { action: 'click->clipboard#copy', clipboard_text_param: recovery_code }
        ) do
          render Icons::Copy.new(size: 18)
        end
      end

      def add_codes_form
        div(class: 'border-t border-outline-variant/30 pt-6') do
          render RubyUI::Form.new(action: rodauth.recovery_codes_path, method: :post, class: 'space-y-6', data_turbo: 'false') do
            render_additional_tags
            authenticity_token_field
            input(type: 'hidden', name: rodauth.add_recovery_codes_param, value: '1')
            password_field
            submit_button
          end
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
            placeholder: t('rodauth.views.add_recovery_codes.password_placeholder')
          },
          error: view_context.rodauth.field_error(rodauth.password_param)
        )
      end

      def submit_button
        render_m3_submit_button(rodauth.recovery_codes_button)
      end
    end
  end
end

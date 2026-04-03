# frozen_string_literal: true

module Views
  module Rodauth
    module LoginPasskeyUiSupport
      private

      def passkey_section_attributes
        {
          id: 'passkey-login-section',
          hidden: true,
          class: 'mt-8 rounded-2xl border border-border bg-surface-container-low p-6'
        }
      end

      def passkey_section_header
        div(class: 'mb-5 flex items-start gap-4') do
          div(class: 'inline-flex h-10 w-10 items-center justify-center rounded-lg bg-primary text-primary-foreground shadow-sm') do
            render Components::Icons::Fingerprint.new(size: 18)
          end
          div(class: 'space-y-1') do
            p(class: 'text-xs font-semibold uppercase tracking-wider text-muted-foreground') { t('sessions.login.passkey_label') }
            p(class: 'text-sm font-semibold text-foreground') { t('sessions.login.passkey_cta') }
            p(class: 'text-xs leading-relaxed text-muted-foreground') { t('sessions.login.passkey_helper') }
          end
        end
      end

      def passkey_trigger_button
        button(**passkey_trigger_attributes) do
          span(class: 'inline-flex items-center justify-center gap-2') do
            render Components::Icons::Key.new(size: 16)
            span { t('sessions.login.passkey_cta') }
          end
        end
      end

      def passkey_trigger_attributes
        {
          type: 'button',
          id: 'passkey-login-trigger',
          hidden: true,
          disabled: true,
          class: 'w-full h-11 rounded-xl bg-primary px-5 text-sm font-semibold text-primary-foreground transition-all hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-50',
          data_error_unsupported: t('sessions.login.passkey_not_supported'),
          data_error_failed: t('sessions.login.passkey_error')
        }
      end

      def passkey_error_message
        p(
          id: 'passkey-login-error',
          hidden: true,
          role: 'status',
          aria_live: 'polite',
          class: 'mt-3 text-xs font-medium text-destructive'
        )
      end
    end
  end
end

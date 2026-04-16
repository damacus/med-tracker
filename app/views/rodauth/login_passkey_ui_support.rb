# frozen_string_literal: true

module Views
  module Rodauth
    module LoginPasskeyUiSupport
      private

      def passkey_section_attributes
        {
          id: 'passkey-login-section',
          hidden: true,
          class: 'mt-10 rounded-[2rem] border border-outline-variant/30 bg-surface-container-low p-8 shadow-elevation-1'
        }
      end

      def passkey_section_header
        div(class: 'mb-6 flex items-start gap-4') do
          div(
            class: 'inline-flex h-12 w-12 items-center justify-center rounded-2xl ' \
                   'bg-primary text-on-primary shadow-elevation-2'
          ) do
            render Components::Icons::Fingerprint.new(size: 22)
          end
          div(class: 'space-y-1') do
            m3_text(variant: :label_small, class: 'uppercase tracking-[0.2em] font-black text-on-surface-variant') do
              t('sessions.login.passkey_label')
            end
            m3_heading(variant: :title_medium, class: 'font-bold text-foreground') { t('sessions.login.passkey_cta') }
            m3_text(variant: :body_small, class: 'text-on-surface-variant font-medium') { t('sessions.login.passkey_helper') }
          end
        end
      end

      def passkey_trigger_button
        button(**passkey_trigger_attributes) do
          span(class: 'inline-flex items-center justify-center gap-2 z-10') do
            render Components::Icons::Key.new(size: 18)
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
          class: 'w-full h-14 rounded-2xl bg-primary px-5 text-sm font-bold ' \
                 'text-on-primary transition-all relative state-layer shadow-lg shadow-primary/20 ' \
                 'disabled:cursor-not-allowed disabled:opacity-38 disabled:shadow-none',
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
          class: 'mt-3 px-1 text-xs font-bold text-error'
        )
      end
    end
  end
end
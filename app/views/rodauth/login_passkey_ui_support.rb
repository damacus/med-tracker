# frozen_string_literal: true

module Views
  module Rodauth
    module LoginPasskeyUiSupport
      private

      def passkey_section_attributes
        {
          id: 'passkey-login-section',
          hidden: true,
          class: 'space-y-2'
        }
      end

      def passkey_section_header
        nil
      end

      def passkey_trigger_button
        button(**passkey_trigger_attributes) do
          span(class: 'flex items-center gap-5') do
            span(class: 'grid h-12 w-12 place-items-center rounded-lg border border-teal-300/60 bg-teal-50 text-teal-600 dark:border-teal-400/35 dark:bg-teal-400/10 dark:text-teal-300') do
              render Components::Icons::Fingerprint.new(size: 24)
            end
            span { t('sessions.login.passkey_cta') }
          end
          render Components::Icons::ChevronRight.new(size: 24)
        end
      end

      def passkey_trigger_attributes
        {
          type: 'button',
          id: 'passkey-login-trigger',
          hidden: true,
          disabled: true,
          class: 'flex h-16 w-full items-center justify-between rounded-lg border border-outline-variant ' \
                 'bg-surface-container-lowest px-2 pr-4 text-left font-bold text-foreground shadow-sm transition ' \
                 'hover:border-teal-500/60 hover:bg-surface-container-low focus-visible:outline-none ' \
                 'focus-visible:ring-2 focus-visible:ring-teal-500/25 disabled:cursor-not-allowed disabled:opacity-38',
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

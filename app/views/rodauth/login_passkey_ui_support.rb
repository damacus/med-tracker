# frozen_string_literal: true

module Views
  module Rodauth
    module LoginPasskeyUiSupport
      private

      def passkey_section_attributes
        {
          id: 'passkey-login-section',
          hidden: true,
          class: 'mt-10 rounded-[2rem] border border-black/10 bg-[linear-gradient(145deg,rgba(255,255,255,0.84),rgba(245,240,231,0.92))] p-5 shadow-[0_24px_50px_-36px_rgba(15,23,42,0.45)] sm:p-6'
        }
      end

      def passkey_section_header
        div(class: 'mb-5 flex items-start gap-4') do
          div(class: 'inline-flex h-12 w-12 items-center justify-center rounded-2xl bg-zinc-950 text-white shadow-sm') do
            render Components::Icons::Fingerprint.new(size: 20)
          end
          div(class: 'space-y-1') do
            p(class: 'text-[0.68rem] font-black uppercase tracking-[0.28em] text-zinc-500') { t('sessions.login.passkey_label') }
            p(class: 'text-sm font-semibold text-zinc-900') { t('sessions.login.passkey_cta') }
            p(class: 'text-sm leading-6 text-zinc-600') { t('sessions.login.passkey_helper') }
          end
        end
      end

      def passkey_trigger_button
        button(**passkey_trigger_attributes) do
          span(class: 'inline-flex items-center justify-center gap-3') do
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
          class: 'w-full rounded-[1.5rem] border border-black/10 bg-zinc-950 px-5 py-5 text-sm font-black text-white shadow-[0_18px_35px_-24px_rgba(24,24,27,0.72)] transition-all duration-300 hover:-translate-y-0.5 hover:bg-zinc-900 disabled:cursor-not-allowed disabled:opacity-60',
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
          class: 'mt-3 text-sm font-semibold text-destructive'
        )
      end
    end
  end
end

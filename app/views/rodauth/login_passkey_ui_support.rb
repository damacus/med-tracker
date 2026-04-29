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
            render_passkey_sign_in_icon
            span { t('sessions.login.passkey_cta') }
          end
          render Components::Icons::ChevronRight.new(
            size: 24,
            path: 'M9 5L16 12L9 19',
            stroke_width: '2.5',
            data_login_sign_in_chevron: 'passkey'
          )
        end
      end

      def render_passkey_sign_in_icon
        svg(width: '48', height: '48', viewbox: '0 0 48 48', fill: 'none', xmlns: 'http://www.w3.org/2000/svg',
            data_login_sign_in_icon: 'passkey', aria_hidden: 'true', class: 'h-12 w-12 shrink-0') do |s|
          s.rect(width: '48', height: '48', rx: '14', fill: '#E0F7F4')
          s.g(transform: 'translate(12 12)') do |g|
            g.path(d: passkey_streamline_path, fill: '#109E91', stroke_width: '0.5')
          end
        end
      end

      def passkey_streamline_path
        'M3 20v-2.35c0 -0.63335 0.158335 -1.175 0.475 -1.625 0.316665 -0.45 0.725 -0.79165 1.225 -1.025 ' \
          '1.11665 -0.5 2.1875 -0.875 3.2125 -1.125S9.96665 13.5 11 13.5c0.43335 0 0.85415 0.02085 ' \
          '1.2625 0.0625s0.82915 0.10415 1.2625 0.1875c-0.08335 0.96665 0.09585 1.87915 0.5375 2.7375C14.50415 ' \
          '17.34585 15.15 18.01665 16 18.5v1.5H3Zm16 3.675 -1.5 -1.5v-4.65c-0.73335 -0.21665 -1.33335 ' \
          '-0.62915 -1.8 -1.2375 -0.46665 -0.60835 -0.7 -1.3125 -0.7 -2.1125 0 -0.96665 0.34165 -1.79165 ' \
          '1.025 -2.475 0.68335 -0.68335 1.50835 -1.025 2.475 -1.025s1.79165 0.34165 2.475 1.025c0.68335 ' \
          '0.68335 1.025 1.50835 1.025 2.475 0 0.75 -0.2125 1.41665 -0.6375 2 -0.425 0.58335 -0.9625 1 ' \
          '-1.6125 1.25l1.25 1.25 -1.5 1.5 1.5 1.5 -2 2ZM11 11.5c-1.05 0 -1.9375 -0.3625 -2.6625 ' \
          '-1.0875 -0.725 -0.725 -1.0875 -1.6125 -1.0875 -2.6625s0.3625 -1.9375 1.0875 -2.6625C9.0625 4.3625 ' \
          '9.95 4 11 4s1.9375 0.3625 2.6625 1.0875c0.725 0.725 1.0875 1.6125 1.0875 2.6625s-0.3625 1.9375 ' \
          '-1.0875 2.6625C12.9375 11.1375 12.05 11.5 11 11.5Zm7.5 3.175c0.28335 0 0.52085 -0.09585 ' \
          '0.7125 -0.2875S19.5 13.95835 19.5 13.675c0 -0.28335 -0.09585 -0.52085 -0.2875 -0.7125s-0.42915 ' \
          '-0.2875 -0.7125 -0.2875c-0.28335 0 -0.52085 0.09585 -0.7125 0.2875S17.5 13.39165 17.5 13.675c0 ' \
          '0.28335 0.09585 0.52085 0.2875 0.7125s0.42915 0.2875 0.7125 0.2875Z'
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

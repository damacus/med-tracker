# frozen_string_literal: true

module Views
  module Rodauth
    module LoginSecondarySignInSupport
      private

      def render_secondary_sign_in_options
        div(**secondary_sign_in_options_attributes) do
          render_oauth_divider
          p(class: 'text-sm font-semibold text-on-surface-variant') { t('sessions.login.other_sign_in_options') }
          div(class: 'space-y-3') do
            render_passkey_section
            render_oauth_button if oauth_enabled? && !invite_only?
          end
        end
      end

      def secondary_sign_in_options_attributes
        attrs = { id: 'secondary-sign-in-options', class: 'mt-9 space-y-4' }
        attrs[:hidden] = true unless oauth_enabled? && !invite_only?
        attrs
      end

      def render_oauth_divider
        div(class: 'flex items-center gap-5 text-sm font-semibold text-on-surface-variant') do
          div(class: 'h-px flex-1 bg-outline-variant/70')
          span { t('sessions.login.oauth_divider') }
          div(class: 'h-px flex-1 bg-outline-variant/70')
        end
      end

      def render_oauth_button
        provider_name = ENV.fetch('OIDC_PROVIDER_NAME', 'OIDC')
        form(action: view_context.rodauth.omniauth_request_path(:oidc), method: 'post', data: { turbo: 'false' }) do
          input(type: 'hidden', name: 'authenticity_token', value: view_context.form_authenticity_token)
          render_oauth_button_content(provider_name)
        end
      end

      def render_oauth_button_content(provider_name)
        button(type: 'submit', class: secondary_sign_in_button_classes) do
          span(class: 'flex items-center gap-5') do
            render_oauth_icon_tile
            span { t('sessions.login.oauth_sso', provider: provider_name) }
          end
          render Components::Icons::ChevronRight.new(size: 24)
        end
      end

      def render_oauth_icon_tile
        span(class: 'grid h-12 w-12 place-items-center rounded-lg border border-purple-300/55 bg-purple-100 text-purple-600 dark:border-purple-400/35 dark:bg-purple-500/15 dark:text-purple-300') do
          render Components::Icons::Globe.new(size: 22)
        end
      end
    end
  end
end

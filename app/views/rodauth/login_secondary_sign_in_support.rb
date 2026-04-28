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
          render Components::Icons::ChevronRight.new(size: 24, data_login_sign_in_chevron: 'sso')
        end
      end

      def render_oauth_icon_tile
        svg(width: '48', height: '48', viewbox: '0 0 48 48', fill: 'none', xmlns: 'http://www.w3.org/2000/svg',
            data_login_sign_in_icon: 'sso', aria_hidden: 'true', class: 'h-12 w-12 shrink-0') do |s|
          s.rect(width: '48', height: '48', rx: '14', fill: '#F2E7FF')
          s.svg(x: '12', y: '12', width: '24', height: '24', viewbox: '0 -960 960 960',
                xmlns: 'http://www.w3.org/2000/svg') do |icon|
            icon.path(d: sso_cloud_lock_path, fill: '#9A5CF7')
          end
        end
      end

      def sso_cloud_lock_path
        'M480-380Zm80 220H260q-91 0-155.5-63T40-377q0-78 47-139t123-78q25-92 100-149t170-57q106 ' \
          '0 184.5 68.5T757-560q-21 0-40.5 4.5T679-543q-8-75-65-126t-134-51q-83 0-141.5 58.5T280-520h-20q-58 ' \
          '0-99 41t-41 99q0 58 41 99t99 41h300v80Zm120 0q-17 0-28.5-11.5T640-200v-120q0-17 ' \
          '11.5-28.5T680-360v-40q0-33 23.5-56.5T760-480q33 0 56.5 23.5T840-400v40q17 0 ' \
          '28.5 11.5T880-320v120q0 17-11.5 28.5T840-160H680Zm40-200h80v-40q0-17-11.5-28.5T760-440q-17 ' \
          '0-28.5 11.5T720-400v40Z'
      end
    end
  end
end

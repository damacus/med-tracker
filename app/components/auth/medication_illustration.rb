# frozen_string_literal: true

module Components
  module Auth
    class MedicationIllustration < Components::Base
      LIGHT_ASSETS = {
        desktop: 'auth/login-med-illustration-light-desktop.webp',
        mobile: 'auth/login-med-illustration-light-mobile.webp'
      }.freeze

      DARK_ASSETS = {
        desktop: 'auth/login-med-illustration-dark-desktop.webp',
        mobile: 'auth/login-med-illustration-dark-mobile.webp'
      }.freeze

      def initialize(label:, image_path_resolver:)
        @label = label
        @image_path_resolver = image_path_resolver
        super()
      end

      def view_template
        div(**illustration_attrs) { render_picture }
        render_activation_script
      end

      private

      attr_reader :label, :image_path_resolver

      def illustration_attrs
        {
          data_login_illustration: 'medication',
          data_login_illustration_light_desktop_src: asset_path(LIGHT_ASSETS.fetch(:desktop)),
          data_login_illustration_light_mobile_src: asset_path(LIGHT_ASSETS.fetch(:mobile)),
          data_login_illustration_dark_desktop_src: asset_path(DARK_ASSETS.fetch(:desktop)),
          data_login_illustration_dark_mobile_src: asset_path(DARK_ASSETS.fetch(:mobile)),
          role: 'img',
          aria_label: label,
          class: 'login-med-illustration'
        }
      end

      def render_picture
        picture(class: 'login-med-illustration__picture') do
          source(media: '(max-width: 520px)', data_login_illustration_source: 'mobile')
          img(
            alt: '',
            aria_hidden: 'true',
            loading: 'eager',
            fetchpriority: 'high',
            decoding: 'async',
            data_login_illustration_image: true,
            class: 'login-med-illustration__image'
          )
        end
      end

      def render_activation_script
        script(nonce: view_context.content_security_policy_nonce) do
          plain 'window.MedTrackerAuth?.applyLoginIllustrations?.()'
        end
      end

      def asset_path(asset)
        image_path_resolver.call(asset)
      end
    end
  end
end

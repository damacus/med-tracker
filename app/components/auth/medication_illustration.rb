# frozen_string_literal: true

module Components
  module Auth
    class MedicationIllustration < Components::Base
      LIGHT_ASSETS = {
        desktop: 'auth/login-med-illustration-light-desktop.png',
        mobile: 'auth/login-med-illustration-light-mobile.png',
        class: 'login-med-illustration__picture--light'
      }.freeze

      DARK_ASSETS = {
        desktop: 'auth/login-med-illustration-dark-desktop.png',
        mobile: 'auth/login-med-illustration-dark-mobile.png',
        class: 'login-med-illustration__picture--dark'
      }.freeze

      def initialize(label:, image_path_resolver:)
        @label = label
        @image_path_resolver = image_path_resolver
        super()
      end

      def view_template
        div(data_login_illustration: 'medication', role: 'img', aria_label: label, class: 'login-med-illustration') do
          render_picture(LIGHT_ASSETS)
          render_picture(DARK_ASSETS)
        end
      end

      private

      attr_reader :label, :image_path_resolver

      def render_picture(assets)
        picture(class: "login-med-illustration__picture #{assets.fetch(:class)}") do
          source(media: '(max-width: 520px)', srcset: image_path_resolver.call(assets.fetch(:mobile)))
          img(
            src: image_path_resolver.call(assets.fetch(:desktop)),
            alt: '',
            aria_hidden: 'true',
            loading: 'eager',
            class: 'login-med-illustration__image'
          )
        end
      end
    end
  end
end

# frozen_string_literal: true

module Views
  module Rodauth
    module LoginBrandSupport
      private

      def render_brand_panel
        div(data_login_panel: 'brand', class: brand_panel_classes) do
          render_brand_header
          render_brand_welcome
          render_benefit_list
          render_medication_illustration
        end
      end

      def render_brand_header
        div(class: 'flex items-center justify-center gap-4 md:justify-start md:gap-5') do
          render_mt_logo
          span(class: 'text-xl font-bold text-foreground') { t('app.name') }
        end
      end

      def render_mt_logo
        render Components::Auth::MtLogo.new(label: t('app.name'))
      end

      def render_brand_welcome
        div(class: 'space-y-2 text-center md:mt-10 md:space-y-3 md:text-left') do
          h1(class: 'text-2xl font-bold leading-tight text-foreground sm:text-4xl md:text-5xl') { t('sessions.login.heading') }
          p(class: 'text-sm font-medium text-on-surface-variant md:text-base') { t('sessions.login.subheading') }
        end
      end

      def render_medication_illustration
        render Components::Auth::MedicationIllustration.new(
          label: t('sessions.login.medication_illustration_label'),
          image_path_resolver: ->(asset_path) { view_context.image_path(asset_path) }
        )
      end
    end
  end
end

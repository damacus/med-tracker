# frozen_string_literal: true

module Components
  module Layouts
    class AuthLayout < Components::Base
      include Phlex::Rails::Helpers::CSPMetaTag
      include Phlex::Rails::Helpers::CSRFMetaTags
      include Phlex::Rails::Helpers::JavaScriptIncludeTag
      include Phlex::Rails::Helpers::StylesheetLinkTag

      def initialize(title: 'Med Tracker', component: nil)
        @title = title
        @component = component
      end

      def view_template(&)
        doctype
        html(lang: I18n.locale, data: { allow_palette: false }) do
          head do
            meta(charset: 'UTF-8')
            title { @title }
            meta(name: 'description', content: 'Med Tracker - Manage your medications and health with ease.')
            meta(name: 'viewport', content: 'width=device-width,initial-scale=1')
            meta(name: 'theme-color', content: '#f8fafc')
            csp_meta_tag
            csrf_meta_tags

            javascript_include_tag 'appearance_boot', 'data-turbo-track': 'reload'
            stylesheet_link_tag 'tailwind', 'data-turbo-track': 'reload'
            javascript_include_tag 'auth', 'data-turbo-track': 'reload'
          end

          body(class: 'bg-background text-foreground') do
            div(id: 'flash')

            main do
              render @component if @component
              yield if block_given?
            end
          end
        end
      end

      private

      def flash
        view_context.flash
      end
    end
  end
end

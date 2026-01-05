# frozen_string_literal: true

module Components
  module Layouts
    class AuthLayout < Components::Base
      include Phlex::Rails::Helpers::CSPMetaTag
      include Phlex::Rails::Helpers::CSRFMetaTags
      include Phlex::Rails::Helpers::StylesheetLinkTag
      include Phlex::Rails::Helpers::JavaScriptImportmapTags

      def initialize(title: 'Med Tracker', component: nil)
        @title = title
        @component = component
      end

      def view_template(&)
        doctype
        html do
          head do
            title { @title }
            meta(name: 'viewport', content: 'width=device-width,initial-scale=1')
            csp_meta_tag
            csrf_meta_tags

            stylesheet_link_tag 'tailwind', 'data-turbo-track': 'reload'
            stylesheet_link_tag 'application', 'data-turbo-track': 'reload'
            javascript_importmap_tags
          end

          body(class: 'bg-slate-50') do
            div(id: 'flash') do
              render Components::Layouts::Flash.new(
                notice: flash[:notice],
                alert: flash[:alert]
              )
            end

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

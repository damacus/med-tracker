# frozen_string_literal: true

module Views
  module Layouts
    # The main application layout.
    class Application < Views::Base
      def initialize(title: 'Med Tracker')
        super()
        @title = title
      end

      def view_template(&block)
        doctype

        html do
          render_head
          render_body(&block)
        end
      end

      private

      def render_head
        head do
          title { @title }
          meta name: 'viewport', content: 'width=device-width,initial-scale=1'
          meta name: 'apple-mobile-web-app-capable', content: 'yes'
          meta name: 'mobile-web-app-capable', content: 'yes'
          meta name: 'theme-color', content: '#007BFF'
          meta name: 'apple-mobile-web-app-status-bar-style', content: 'black-translucent'

          csrf_meta_tags
          csp_meta_tag

          yield :head

          link rel: 'manifest', href: '/manifest.json'
          favicon_link_tag '/favicon.svg', type: 'image/svg+xml'
          favicon_link_tag '/favicon.svg', rel: 'apple-touch-icon', type: 'image/svg+xml'
          stylesheet_link_tag 'application', 'data-turbo-track': 'reload'
          javascript_importmap_tags
          javascript_include_tag 'application', 'data-turbo-track': 'reload', type: 'module'
        end
      end

      def render_body(&block)
        body do
          render Components::Layouts::Navigation.new

          div id: 'flash' do
            render Views::Shared::Flash.new(flash: flash)
          end

          service_worker_script

          main(&block)

          div id: 'modal'
        end
      end

      def service_worker_script
        script do
          plain <<~JS.squish
            if ('serviceWorker' in navigator) {
              window.addEventListener('load', function() {
                navigator.serviceWorker.register('/service-worker.js')
                  .then(function(registration) {
                    console.log('ServiceWorker registration successful');
                  })
                  .catch(function(err) {
                    console.log('ServiceWorker registration failed: ', err);
                  });
              });
            }
          JS
        end
      end
    end
  end
end


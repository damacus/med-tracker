# frozen_string_literal: true

module Views
  module Mailers
    class Layout < Views::Base
      include Phlex::Rails::Helpers::StylesheetLinkTag

      def view_template(&)
        doctype
        html(lang: I18n.locale) do
          head
          body(class: 'mailer-body') do
            div(class: 'mailer-wrapper') do
              div(class: 'mailer-card') do
                header(class: 'mailer-header') do
                  p(class: 'mailer-brand') { I18n.t('app.name') }
                end
                main(class: 'mailer-content', &)
                footer(class: 'mailer-footer') do
                  plain "#{I18n.t('app.name')} - #{I18n.t('layouts.mailer.footer_notice')}"
                end
              end
            end
          end
        end
      end

      private

      def head
        super do
          meta(name: 'viewport', content: 'width=device-width, initial-scale=1.0')
          title { I18n.t('app.name') }
          stylesheet_link_tag 'mailer'
        end
      end
    end
  end
end

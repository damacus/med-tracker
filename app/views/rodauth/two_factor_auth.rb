# frozen_string_literal: true

module Views
  module Rodauth
    class TwoFactorAuth < Views::Rodauth::Base
      def view_template
        page_layout do
          render_auth_card(
            title: t('rodauth.views.two_factor_auth.page_title'),
            subtitle: t('rodauth.views.two_factor_auth.page_subtitle')
          ) do
            render_method_links
          end
        end
      end

      private

      def render_method_links
        div(class: 'flex flex-col gap-4') do
          rodauth.two_factor_auth_links.each do |_, link, text|
            m3_link(variant: :outlined, size: :lg, href: link, class: 'w-full py-6 font-bold bg-surface-container-low') do
              text
            end
          end
        end
      end
    end
  end
end

# frozen_string_literal: true

module Views
  module Rodauth
    class TwoFactorAuth < Views::Rodauth::Base
      def view_template
        page_layout do
          render_page_header(
            title: 'Additional authentication required',
            subtitle: 'Choose an available method to confirm your identity.'
          )
          links_section
        end
      end

      private

      def links_section
        render_card_section(
          title: 'Verify with',
          description: 'Select one of the available authentication methods.'
        ) { method_links }
      end

      def method_links
        rodauth.two_factor_auth_links.each do |_, link, text|
          render RubyUI::Link.new(variant: :outline, size: :lg, href: link, class: 'w-full') do
            text
          end
        end
      end
    end
  end
end

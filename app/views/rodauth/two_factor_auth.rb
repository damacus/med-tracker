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
        render RubyUI::Card.new(class: card_classes) do
          render RubyUI::CardHeader.new(class: 'space-y-2 bg-white/60') do
            render RubyUI::CardTitle.new(class: 'text-xl font-semibold text-slate-900') do
              'Verify with'
            end
            render RubyUI::CardDescription.new(class: 'text-base text-slate-600') do
              'Select one of the available authentication methods.'
            end
          end
          render RubyUI::CardContent.new(class: 'space-y-4 p-6 sm:p-8') do
            method_links
          end
        end
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

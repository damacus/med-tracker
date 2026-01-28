# frozen_string_literal: true

module Views
  module Rodauth
    class TwoFactorAuth < Views::Base
      def view_template
        div(class: 'relative min-h-screen bg-gradient-to-br from-sky-50 via-white to-indigo-100 py-16 sm:py-20') do
          decorative_glow

          div(class: 'relative mx-auto flex w-full max-w-2xl flex-col items-center gap-8 px-4 sm:px-6 lg:px-8') do
            header_section
            links_section
          end
        end
      end

      private

      def header_section
        div(class: 'mx-auto max-w-xl text-center space-y-3') do
          h1(class: 'text-3xl font-bold tracking-tight text-slate-800 sm:text-4xl') do
            'Additional authentication required'
          end
          p(class: 'text-lg text-slate-600') do
            'Choose an available method to confirm your identity.'
          end
        end
      end

      def decorative_glow
        div(class: 'pointer-events-none absolute inset-x-0 top-24 flex justify-center opacity-60') do
          div(class: 'h-64 w-64 rounded-full bg-sky-200 blur-3xl sm:h-80 sm:w-80')
        end
      end

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

      def card_classes
        'w-full backdrop-blur bg-white/90 shadow-2xl border border-white/70 ring-1 ring-black/5 rounded-2xl'
      end

      def rodauth
        view_context.rodauth
      end
    end
  end
end

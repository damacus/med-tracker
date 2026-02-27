# frozen_string_literal: true

module Views
  module Rodauth
    class Base < Views::Base
      include Phlex::Rails::Helpers::TurboFrameTag

      CARD_CLASSES = 'w-full backdrop-blur bg-white/90 shadow-2xl border border-white/70 ring-1 ring-black/5 rounded-2xl'
      PAGE_CLASSES = 'relative min-h-screen bg-gradient-to-br from-sky-50 via-white to-indigo-100 py-16 sm:py-20'
      CONTENT_WRAPPER_CLASSES = 'relative mx-auto flex w-full max-w-2xl flex-col items-center gap-8 px-4 sm:px-6 lg:px-8'
      HEADER_WRAPPER_CLASSES = 'mx-auto max-w-xl text-center space-y-3'
      TITLE_CLASSES = 'text-3xl font-bold tracking-tight text-slate-800 sm:text-4xl'
      SUBTITLE_CLASSES = 'text-lg text-slate-600'

      private

      def page_layout(&block)
        if helpers.request.headers['Turbo-Frame'] == 'modal'
          turbo_frame_tag 'modal' do
            render ::Components::Modal.new(title: @page_title || title) do
              div(class: 'p-4') { block.call }
            end
          end
        else
          div(class: PAGE_CLASSES) do
            decorative_glow
            div(class: CONTENT_WRAPPER_CLASSES, &block)
          end
        end
      end

      def title
        if rodauth.current_route == :change_password
          rodauth.change_password_button
        elsif rodauth.current_route == :change_login
          rodauth.change_login_button
        else
          'Authentication'
        end
      end

      def render_page_header(title:, subtitle:)
        div(class: HEADER_WRAPPER_CLASSES) do
          h1(class: TITLE_CLASSES) { title }
          p(class: SUBTITLE_CLASSES) { subtitle }
        end
      end

      def decorative_glow
        div(class: 'pointer-events-none absolute inset-x-0 top-24 flex justify-center opacity-60') do
          div(class: 'h-64 w-64 rounded-full bg-sky-200 blur-3xl sm:h-80 sm:w-80')
        end
      end

      def card_classes
        CARD_CLASSES
      end

      def authenticity_token_field
        input(type: 'hidden', name: 'authenticity_token', value: view_context.form_authenticity_token)
      end

      def rodauth
        view_context.rodauth
      end
    end
  end
end

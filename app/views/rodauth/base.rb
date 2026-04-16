# frozen_string_literal: true

module Views
  module Rodauth
    class Base < Views::Base
      include Phlex::Rails::Helpers::TurboFrameTag
      include Components::M3Helpers

      PAGE_CLASSES = 'relative min-h-screen bg-surface-container-lowest py-16 sm:py-20 flex flex-col items-center'
      CONTENT_WRAPPER_CLASSES = 'relative mx-auto flex w-full max-w-2xl flex-col items-center gap-8 px-4 sm:px-6 lg:px-8 z-10'
      HEADER_WRAPPER_CLASSES = 'mx-auto max-w-xl text-center space-y-2'

      private

      def page_layout(&block)
        if view_context.request.headers['Turbo-Frame'] == 'modal'
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
          m3_heading(variant: :display_small, level: 1, class: 'font-black tracking-tight') { title }
          m3_text(variant: :body_large, class: 'text-on-surface-variant font-medium') { subtitle }
        end
      end

      def decorative_glow
        div(class: 'pointer-events-none absolute inset-x-0 top-24 flex justify-center opacity-40') do
          div(class: 'h-64 w-64 rounded-full bg-primary/10 blur-3xl sm:h-80 sm:w-80')
        end
      end

      def render_auth_card(title: nil, subtitle: nil, &block)
        m3_card(variant: :elevated, class: 'w-full rounded-[2.5rem] border-none shadow-elevation-3 overflow-visible') do
          if title || subtitle
            m3_card_header(class: 'space-y-2 pb-2 pt-8 px-8 md:px-10') do
              m3_card_title(class: 'text-3xl font-black tracking-tight text-foreground') { title } if title
              if subtitle
                m3_card_description(variant: :body_large, class: 'text-on-surface-variant font-medium') { subtitle }
              end
            end
          end

          m3_card_content(class: 'space-y-6 p-8 md:p-10') do
            block.call if block_given?
          end
        end
      end

      def render_m3_form_field(label:, input_attrs:, error: nil, actions: nil)
        div(class: 'space-y-2') do
          div(class: 'flex items-center justify-between px-1') do
            render RubyUI::FormFieldLabel.new(for: input_attrs[:id], class: 'text-[10px] font-black uppercase tracking-widest text-on-surface-variant') { label }
            actions&.call
          end

          # Standardize input styling for auth forms
          input_attrs[:class] = "h-14 rounded-2xl bg-surface-container-lowest border-outline-variant focus:ring-2 focus:ring-primary/10 transition-all #{input_attrs[:class]}"
          m3_input(**input_attrs)
          p(class: 'mt-1 px-1 text-sm text-error font-medium') { error } if error.present?
        end
      end

      def render_m3_submit_button(label)
        m3_button(type: :submit, variant: :filled, size: :lg, class: 'w-full py-6 font-bold shadow-lg shadow-primary/20') do
          label
        end
      end

      def render_m3_alert(message, variant: :destructive)
        render RubyUI::Alert.new(variant: variant, class: 'rounded-2xl border-none shadow-sm') do
          div(class: 'flex items-center gap-2') do
            render Icons::AlertCircle.new(size: 18)
            span(class: 'font-medium') { message }
          end
        end
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

# frozen_string_literal: true

module Views
  module Rodauth
    class VerifyAccount < Views::Base
      include Phlex::Rails::Helpers::FormWith
      include Phlex::Rails::Helpers::LinkTo

      def view_template
        div(class: 'relative min-h-screen bg-gradient-to-br from-sky-50 via-white to-indigo-100 py-16 sm:py-20') do
          decorative_glow

          div(class: 'relative mx-auto flex w-full max-w-5xl flex-col items-center gap-12 px-4 sm:px-6 lg:px-8') do
            header_section
            form_section
          end
        end
      end

      private

      def header_section
        div(class: 'mx-auto max-w-xl text-center space-y-3') do
          h1(class: 'text-4xl font-bold tracking-tight text-slate-800 sm:text-5xl') { 'MedTracker' }
          p(class: 'text-lg text-slate-600 sm:text-xl') do
            'Verify your account to complete registration.'
          end
        end
      end

      def flash_section
        return if flash_message.blank?

        div(id: 'verify-flash') do
          render RubyUI::Alert.new(variant: flash_variant) do
            plain(flash_message)
          end
        end
      end

      def flash_message
        view_context.flash[:alert] || view_context.flash[:notice]
      end

      def flash_variant
        view_context.flash[:alert].present? ? :destructive : :success
      end

      def form_section
        render RubyUI::Card.new(class: card_classes) do
          render_card_header
          render_card_content
        end
      end

      def card_classes
        'w-full max-w-xl backdrop-blur bg-white/90 shadow-2xl border border-white/70 ' \
          'ring-1 ring-black/5 rounded-2xl overflow-hidden'
      end

      def render_card_header
        render RubyUI::CardHeader.new(class: 'space-y-2 bg-white/60') do
          render RubyUI::CardTitle.new(class: 'text-2xl font-semibold text-slate-900') { 'Verify Your Account' }
          render RubyUI::CardDescription.new(class: 'text-base text-slate-600') do
            plain 'Click the button below to verify your email address and activate your account.'
          end
        end
      end

      def render_card_content
        render RubyUI::CardContent.new(class: 'space-y-6 p-6 sm:p-8') do
          flash_section
          render_verify_form
        end
      end

      def render_verify_form
        render RubyUI::Form.new(action: view_context.rodauth.verify_account_path, method: :post, class: 'space-y-6', data_turbo: 'false') do
          authenticity_token_field
          key_field
          submit_button
        end
      end

      def decorative_glow
        div(class: 'pointer-events-none absolute inset-x-0 top-24 flex justify-center opacity-60') do
          div(class: 'h-64 w-64 rounded-full bg-sky-200 blur-3xl sm:h-80 sm:w-80')
        end
      end

      def authenticity_token_field
        input(type: 'hidden', name: 'authenticity_token', value: view_context.form_authenticity_token)
      end

      def key_field
        input(type: 'hidden', name: 'key', value: view_context.params[:key])
      end

      def submit_button
        render RubyUI::Button.new(type: :submit, variant: :primary, size: :md, class: 'w-full') do
          'Verify Account'
        end
      end
    end
  end
end

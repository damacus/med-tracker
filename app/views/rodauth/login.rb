# frozen_string_literal: true

module Views
  module Rodauth
    class Login < Views::Base
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
            'Sign in to manage your medication plan and keep every dose on track.'
          end
        end
      end

      def flash_section
        return if flash_message.blank?

        div(id: 'login-flash') do
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
          render RubyUI::CardTitle.new(class: 'text-2xl font-semibold text-slate-900') { 'Welcome back' }
          render RubyUI::CardDescription.new(class: 'text-base text-slate-600') do
            plain 'Enter your credentials to access your personalized medication dashboard.'
          end
        end
      end

      def render_card_content
        render RubyUI::CardContent.new(class: 'space-y-6 p-6 sm:p-8') do
          flash_section
          render_login_form
          render_other_options
        end
      end

      def render_login_form
        render RubyUI::Form.new(action: view_context.rodauth.login_path, method: :post, class: 'space-y-6', data_turbo: 'false') do
          authenticity_token_field
          email_field
          password_field
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

      def email_field
        render_form_field(label: 'Email address', input_attrs: email_input_attrs)
      end

      def password_field
        render_form_field(label: 'Password', input_attrs: password_input_attrs) do
          render RubyUI::Link.new(href: view_context.rodauth.reset_password_request_path, variant: :link, size: :sm) do
            'Forgot password?'
          end
        end
      end

      def render_form_field(label:, input_attrs:, &block)
        render RubyUI::FormField.new do
          if block
            div(class: 'flex items-center justify-between') do
              render RubyUI::FormFieldLabel.new(for: input_attrs[:id]) { label }
              block.call
            end
          else
            render RubyUI::FormFieldLabel.new(for: input_attrs[:id]) { label }
          end

          render RubyUI::Input.new(**input_attrs)
        end
      end

      def email_input_attrs
        {
          type: :email,
          name: 'email',
          id: 'email',
          required: true,
          autofocus: true,
          autocomplete: 'username',
          placeholder: 'Enter your email address',
          value: view_context.params[:email]
        }
      end

      def password_input_attrs
        {
          type: :password,
          name: 'password',
          id: 'password',
          required: true,
          autocomplete: 'current-password',
          placeholder: 'Enter your password',
          maxlength: 72
        }
      end

      def submit_button
        render RubyUI::Button.new(type: :submit, variant: :primary, size: :md, class: 'w-full') { 'Login' }
      end

      def render_other_options
        div(class: 'space-y-3 border-t border-slate-200 pt-6') do
          h3(class: 'text-sm font-medium text-slate-700') { 'Other Options' }
          div(class: 'flex flex-col gap-2 text-sm') do
            render RubyUI::Link.new(href: view_context.rodauth.create_account_path, variant: :link) do
              'Create a New Account'
            end
            render RubyUI::Link.new(href: view_context.rodauth.verify_account_resend_path, variant: :link) do
              'Resend Verify Account Information'
            end
          end
        end
      end
    end
  end
end

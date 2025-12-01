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
          remember_me_field
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

      def remember_me_field
        div(class: 'flex items-center gap-2') do
          input(
            type: 'checkbox',
            name: 'remember',
            id: 'remember',
            value: 't',
            class: 'h-4 w-4 rounded border-slate-300 text-sky-600 focus:ring-sky-500'
          )
          label(for: 'remember', class: 'text-sm text-slate-600') { 'Remember me' }
        end
      end

      def submit_button
        render RubyUI::Button.new(type: :submit, variant: :primary, size: :md, class: 'w-full') { 'Login' }
      end

      def render_other_options
        div(class: 'space-y-4 border-t border-slate-200 pt-6') do
          render_oauth_buttons if oauth_enabled?
          render_account_links
        end
      end

      def oauth_enabled?
        return false unless view_context.rodauth.respond_to?(:omniauth_request_path)

        # Only show OAuth button if credentials are actually configured
        Rails.application.credentials.dig(:google, :client_id).present? ||
          ENV['GOOGLE_CLIENT_ID'].present?
      end

      def render_oauth_buttons
        div(class: 'space-y-3') do
          h3(class: 'text-sm font-medium text-slate-700') { 'Or sign in with' }
          form(action: view_context.rodauth.omniauth_request_path(:google_oauth2), method: 'post', data: { turbo: 'false' }) do
            input(type: 'hidden', name: 'authenticity_token', value: view_context.form_authenticity_token)
            button(
              type: 'submit',
              class: 'flex w-full items-center justify-center gap-3 rounded-lg border border-slate-300 bg-white px-4 py-2.5 text-sm font-medium text-slate-700 shadow-sm hover:bg-slate-50 focus:outline-none focus:ring-2 focus:ring-sky-500 focus:ring-offset-2'
            ) do
              google_icon
              span { 'Continue with Google' }
            end
          end
        end
      end

      def google_icon
        svg(
          class: 'h-5 w-5',
          viewBox: '0 0 24 24',
          xmlns: 'http://www.w3.org/2000/svg'
        ) do |s|
          s.path(
            d: 'M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.09z',
            fill: '#4285F4'
          )
          s.path(
            d: 'M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z',
            fill: '#34A853'
          )
          s.path(
            d: 'M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.07H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.93l2.85-2.22.81-.62z',
            fill: '#FBBC05'
          )
          s.path(
            d: 'M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.07l3.66 2.84c.87-2.6 3.3-4.53 6.16-4.53z',
            fill: '#EA4335'
          )
        end
      end

      def render_account_links
        div(class: 'space-y-3') do
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

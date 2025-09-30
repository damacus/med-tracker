# frozen_string_literal: true

module Views
  module Sessions
    class New < Views::Base # :nodoc:
      include Phlex::Rails::Helpers::FormWith
      include Phlex::Rails::Helpers::LinkTo

      def initialize(params: {}, flash: {})
        super()
        @params = params
        @flash = flash
      end

      def view_template
        div(class: 'relative min-h-screen bg-gradient-to-br from-sky-50 via-white to-indigo-100 py-16 sm:py-20') do
          decorative_glow

          div(class: 'relative mx-auto flex w-full max-w-5xl flex-col items-center gap-12 px-4 sm:px-6 lg:px-8') do
            header_section
            flash_section
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

        render RubyUI::Alert.new(variant: flash_variant) do
          plain(flash_message)
        end
      end

      def flash_message
        @flash[:alert] || @flash[:notice]
      end

      def flash_variant
        @flash[:alert].present? ? :destructive : :success
      end

      def form_section
        render RubyUI::Card.new(class: 'w-full max-w-xl backdrop-blur bg-white/90 shadow-2xl border border-white/70 ring-1 ring-black/5 rounded-2xl overflow-hidden') do
          render RubyUI::CardHeader.new(class: 'space-y-2 bg-white/60') do
            render RubyUI::CardTitle.new(class: 'text-2xl font-semibold text-slate-900') { 'Welcome back' }
            render RubyUI::CardDescription.new(class: 'text-base text-slate-600') { 'Enter your credentials to access your personalized medication dashboard.' }
          end

          render RubyUI::CardContent.new(class: 'space-y-6 p-6 sm:p-8') do
            render RubyUI::Form.new(action: session_path, method: :post, class: 'space-y-6') do
              authenticity_token_field
              email_field
              password_field
              submit_button
            end
          end
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
          render RubyUI::Link.new(href: new_password_path, variant: :link, size: :sm) { 'Forgot password?' }
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
          name: 'email_address',
          id: 'email_address',
          required: true,
          autofocus: true,
          autocomplete: 'username',
          placeholder: 'Enter your email address',
          value: @params[:email_address]
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
        render RubyUI::Button.new(type: :submit, variant: :primary, size: :md, class: 'w-full') { 'Sign in' }
      end
    end
  end
end

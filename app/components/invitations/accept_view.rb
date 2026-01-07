# frozen_string_literal: true

module Components
  module Invitations
    class AcceptView < Components::Base
      include Phlex::Rails::Helpers::FormWith

      def initialize(invitation:)
        @invitation = invitation
      end

      def view_template
        div(class: 'relative min-h-screen bg-gradient-to-br from-sky-50 via-white to-indigo-100 py-16 sm:py-20') do
          div(class: 'relative mx-auto flex w-full max-w-5xl flex-col items-center gap-12 px-4 sm:px-6 lg:px-8') do
            header_section
            form_section
          end
        end
      end

      private

      def header_section
        div(class: 'mx-auto max-w-xl text-center space-y-3') do
          Heading(level: 1, class: 'text-4xl font-bold tracking-tight text-slate-800 sm:text-5xl') { 'MedTracker' }
          Text(size: 'lg', class: 'text-slate-600 sm:text-xl') do
            "Welcome! You've been invited as a #{@invitation.role.titleize}."
          end
        end
      end

      def form_section
        card_classes = 'w-full max-w-xl backdrop-blur bg-white/90 shadow-2xl border border-white/70 ' \
                       'ring-1 ring-black/5 rounded-2xl overflow-hidden'

        Card(class: card_classes) do
          CardHeader(class: 'space-y-2 bg-white/60') do
            CardTitle(class: 'text-2xl font-semibold text-slate-900') { 'Complete Your Account' }
            CardDescription(class: 'text-base text-slate-600') do
              plain 'Fill in your details to accept the invitation.'
            end
          end

          CardContent(class: 'space-y-6 p-6 sm:p-8') do
            render_signup_form
          end
        end
      end

      def render_signup_form
        # Post to Rodauth's create account path, but include the invitation token
        form_with(url: view_context.rodauth.create_account_path, method: :post, class: 'space-y-6',
                  data: { turbo: false }) do
          input(type: 'hidden', name: 'invitation_token', value: @invitation.token)

          FormField do
            FormFieldLabel(for: 'name') { 'Name' }
            Input(type: :text, name: 'name', id: 'name', required: true, autofocus: true,
                  placeholder: 'Enter your full name')
          end

          FormField do
            FormFieldLabel(for: 'date_of_birth') { 'Date of birth' }
            Input(type: :date, name: 'date_of_birth', id: 'date_of_birth', required: true)
          end

          FormField do
            FormFieldLabel(for: 'email') { 'Email' }
            Input(type: :email, name: 'email', id: 'email', value: @invitation.email,
                  readonly: true, class: 'bg-slate-50')
          end

          FormField do
            FormFieldLabel(for: 'password') { 'Password' }
            Input(type: :password, name: 'password', id: 'password', required: true, minlength: 12)
          end

          FormField do
            FormFieldLabel(for: 'password-confirm') { 'Confirm Password' }
            Input(type: :password, name: 'password-confirm', id: 'password-confirm', required: true,
                  minlength: 12)
          end

          Button(type: :submit, variant: :primary, size: :md, class: 'w-full') { 'Create Account' }
        end
      end
    end
  end
end

# frozen_string_literal: true

module Components
  module Invitations
    class AcceptView < Components::Base
      include Phlex::Rails::Helpers::FormWith

      def initialize(invitation:, token:)
        @invitation = invitation
        @token = token
      end

      def view_template
        div(class: 'relative min-h-screen bg-gradient-to-br from-background via-background to-muted py-16 sm:py-20') do
          div(class: 'relative mx-auto flex w-full max-w-5xl flex-col items-center gap-12 px-4 sm:px-6 lg:px-8') do
            header_section
            form_section
          end
        end
      end

      private

      def header_section
        div(class: 'mx-auto max-w-xl text-center space-y-3') do
          Heading(level: 1, class: 'text-4xl font-bold tracking-tight text-foreground sm:text-5xl') { t('app.name') }
          Text(size: 'lg', class: 'text-muted-foreground sm:text-xl') do
            t('invitations.accept.welcome', role: t("activerecord.attributes.invitation.roles.#{@invitation.role}"))
          end
        end
      end

      def form_section
        card_classes = 'w-full max-w-xl overflow-hidden rounded-2xl border border-border/70 bg-card/85 ' \
                       'shadow-2xl ring-1 ring-ring/10 backdrop-blur'

        Card(class: card_classes) do
          CardHeader(class: 'space-y-2 bg-card/60') do
            CardTitle(class: 'text-2xl font-semibold text-foreground') { t('invitations.accept.title') }
            CardDescription(class: 'text-base text-muted-foreground') do
              plain t('invitations.accept.description')
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
          input(type: 'hidden', name: 'invitation_token', value: @token)

          FormField do
            FormFieldLabel(for: 'name') { t('invitations.accept.form.name') }
            Input(type: :text, name: 'name', id: 'name', required: true, autofocus: true,
                  placeholder: t('invitations.accept.form.name_placeholder'))
          end

          FormField do
            FormFieldLabel(for: 'date_of_birth') { t('invitations.accept.form.date_of_birth') }
            Input(type: :date, name: 'date_of_birth', id: 'date_of_birth', required: true)
          end

          FormField do
            FormFieldLabel(for: 'email') { t('invitations.accept.form.email') }
            Input(type: :email, name: 'email', id: 'email', value: @invitation.email,
                  readonly: true, class: 'bg-muted/70')
          end

          FormField do
            FormFieldLabel(for: 'password') { t('invitations.accept.form.password') }
            Input(type: :password, name: 'password', id: 'password', required: true, minlength: 12)
          end

          FormField do
            FormFieldLabel(for: 'password-confirm') { t('invitations.accept.form.password_confirmation') }
            Input(type: :password, name: 'password-confirm', id: 'password-confirm', required: true,
                  minlength: 12)
          end

          Button(type: :submit, variant: :primary, size: :md, class: 'w-full') { t('invitations.accept.form.submit') }
        end
      end
    end
  end
end

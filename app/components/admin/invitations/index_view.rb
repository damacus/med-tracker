# frozen_string_literal: true

module Components
  module Admin
    module Invitations
      class IndexView < Components::Base
        include Phlex::Rails::Helpers::FormWith
        include Components::FormHelpers

        def initialize(invitation: Invitation.new)
          @invitation = invitation
        end

        def view_template
          div(class: 'container mx-auto px-4 py-8 max-w-2xl space-y-8') do
            render_header
            render_errors if @invitation.errors.any?

            Card do
              CardContent(class: 'pt-6') do
                render_form
              end
            end
          end
        end

        private

        def render_errors
          Alert(variant: :destructive) do
            ul do
              @invitation.errors.full_messages.each do |message|
                li { message }
              end
            end
          end
        end

        def render_header
          div(class: 'space-y-2') do
            Heading(level: 1) { 'Invitations' }
            Text(weight: 'muted') { 'Invite new users to join MedTracker.' }
          end
        end

        def render_form
          form_with(url: admin_invitations_path, method: :post, class: 'space-y-6', data: { turbo: false }) do
            render_email_field
            render_role_field
            render_actions
          end
        end

        def render_email_field
          FormField do
            FormFieldLabel(for: 'invitation_email') { 'Email' }
            Input(
              type: :email,
              name: 'invitation[email]',
              id: 'invitation_email',
              required: true
            )
          end
        end

        def render_role_field
          FormField do
            FormFieldLabel(for: 'invitation_role') { 'Role' }
            select(name: 'invitation[role]', id: 'invitation_role', class: select_classes, required: true) do
              User.roles.each_key do |role|
                option(value: role) { role.titleize }
              end
            end
          end
        end

        def render_actions
          div(class: 'flex items-center justify-end') do
            Button(type: :submit, variant: :primary) { 'Send invitation' }
          end
        end
      end
    end
  end
end

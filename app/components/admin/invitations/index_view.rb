# frozen_string_literal: true

module Components
  module Admin
    module Invitations
      class IndexView < Components::Base
        include Phlex::Rails::Helpers::FormWith
        include Phlex::Rails::Helpers::Pluralize
        include Components::FormHelpers

        def initialize(invitation: Invitation.new, invitations: Invitation.order(created_at: :desc))
          @invitation = invitation
          @invitations = invitations
        end

        def view_template
          div(id: 'admin_invitations', class: 'container mx-auto px-4 py-8 pb-24 md:pb-8 max-w-6xl space-y-8') do
            render_header
            render_errors if @invitation.errors.any?

            div(class: 'max-w-2xl mx-auto w-full') do
              Card(class: 'overflow-hidden border-none shadow-2xl rounded-[2.5rem] bg-white') do
                div(class: 'p-10') do
                  render_form
                end
              end
            end

            render_existing_invitations
          end
        end

        private

        def render_header
          div(class: 'flex flex-col md:flex-row md:items-end justify-between gap-6 mb-12') do
            div do
              Text(size: '2', weight: 'muted', class: 'uppercase tracking-widest mb-1 block font-bold') do
                Time.current.strftime('%A, %b %d')
              end
              Heading(level: 1, size: '8', class: 'font-extrabold tracking-tight') do
                'Invitations'
              end
              Text(weight: 'muted', class: 'mt-2 block') { 'Invite new users to join MedTracker.' }
            end
          end
        end

        def render_form
          form_with(url: admin_invitations_path, method: :post, class: 'space-y-8') do
            div(class: 'space-y-6') do
              render_email_field
              render_role_field
            end
            render_actions
          end
        end

        def render_email_field
          FormField(class: 'space-y-2') do
            FormFieldLabel(for: 'invitation_email',
                           class: 'text-[10px] font-black uppercase tracking-widest text-slate-400 ml-1') do
              'Email'
            end
            Input(
              type: :email,
              name: 'invitation[email]',
              id: 'invitation_email',
              value: @invitation.email,
              required: true,
              class: 'rounded-md border-slate-200 bg-white py-4 px-4 focus:ring-2 ' \
                     'focus:ring-primary/10 focus:border-primary transition-all'
            )
          end
        end

        def render_role_field
          FormField(class: 'space-y-2') do
            FormFieldLabel(for: 'invitation_role',
                           class: 'text-[10px] font-black uppercase tracking-widest text-slate-400 ml-1') do
              'Role'
            end
            select(name: 'invitation[role]', id: 'invitation_role', class: select_classes, required: true) do
              Invitation.assignable_roles.each_key do |role|
                option(value: role, selected: @invitation.role == role) { role.titleize }
              end
            end
          end
        end

        def render_actions
          div(class: 'flex items-center justify-end pt-4') do
            Button(type: :submit, variant: :primary, size: :lg,
                   class: 'px-8 rounded-2xl shadow-lg shadow-primary/20') do
              'Send invitation'
            end
          end
        end

        def render_existing_invitations
          return if @invitations.empty?

          div(class: 'max-w-4xl mx-auto w-full') do
            Card(class: 'overflow-hidden border-none shadow-xl rounded-[2rem] bg-white') do
              div(class: 'px-8 py-6 border-b border-slate-100') do
                Heading(level: 2, size: '5', class: 'font-bold tracking-tight') { 'Recent invitations' }
              end

              div(class: 'divide-y divide-slate-100') do
                @invitations.each do |invitation|
                  render_invitation_row(invitation)
                end
              end
            end
          end
        end

        def render_invitation_row(invitation)
          div(class: 'px-8 py-5 flex flex-col md:flex-row md:items-center md:justify-between gap-4') do
            div(class: 'space-y-1') do
              p(class: 'text-sm font-semibold text-slate-900') { invitation.email }
              p(class: 'text-sm text-slate-500') do
                plain "#{invitation.role.titleize} • #{invitation_status_label(invitation)}"
              end
              p(class: 'text-xs text-slate-400') { invitation_metadata(invitation) }
            end

            next unless invitation.resendable?

            form_with(url: resend_admin_invitation_path(invitation), method: :post, class: 'shrink-0') do
              Button(type: :submit, variant: :outline, size: :sm, class: 'rounded-xl') { 'Resend' }
            end
          end
        end

        def invitation_status_label(invitation)
          return 'Accepted' if invitation.accepted?
          return 'Expired' if invitation.expired?

          'Pending'
        end

        def invitation_metadata(invitation)
          if invitation.accepted?
            "Accepted #{view_context.time_ago_in_words(invitation.accepted_at)} ago"
          else
            "Expires #{view_context.time_ago_in_words(invitation.expires_at)} from now"
          end
        end

        def render_errors
          render RubyUI::Alert.new(variant: :destructive, class: 'mb-6') do
            div do
              Heading(level: 2, size: '3', class: 'font-semibold mb-2') do
                plain "#{pluralize(@invitation.errors.count, 'error')} prevented this invitation from being saved:"
              end
              ul(class: 'my-2 ml-6 list-disc [&>li]:mt-1') do
                @invitation.errors.full_messages.each do |message|
                  li { message }
                end
              end
            end
          end
        end
      end
    end
  end
end

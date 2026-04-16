# frozen_string_literal: true

module Components
  module Admin
    module Invitations
      class IndexView < Components::Base
        include Phlex::Rails::Helpers::FormWith
        include Phlex::Rails::Helpers::Pluralize
        include Components::FormHelpers

        def initialize(
          invitation: Invitation.new,
          invitations: Invitation.order(created_at: :desc),
          resendable_invitation_ids: []
        )
          @invitation = invitation
          @invitations = invitations
          @resendable_invitation_ids = resendable_invitation_ids
        end

        def view_template
          div(id: 'admin_invitations', class: 'container mx-auto px-4 py-8 pb-24 md:pb-8 max-w-6xl space-y-8') do
            render_header
            render_errors if @invitation.errors.any?

            div(class: 'max-w-2xl mx-auto w-full') do
              m3_card(class: 'overflow-hidden border-none shadow-2xl rounded-[2.5rem] bg-card') do
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
              m3_text(size: '2', weight: 'muted', class: 'uppercase tracking-widest mb-1 block font-bold') do
                Time.current.strftime('%A, %b %d')
              end
              m3_heading(level: 1, size: '8', class: 'font-extrabold tracking-tight') do
                t('admin.invitations.index.title')
              end
              m3_text(weight: 'muted', class: 'mt-2 block') { t('admin.invitations.index.subtitle') }
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
                           class: 'text-[10px] font-black uppercase tracking-widest text-on-surface-variant ml-1') do
              t('admin.invitations.index.form.email')
            end
            m3_input(
              type: :email,
              name: 'invitation[email]',
              id: 'invitation_email',
              value: @invitation.email,
              required: true,
              class: 'rounded-md border-border bg-card py-4 px-4 focus:ring-2 ' \
                     'focus:ring-primary/10 focus:border-primary transition-all'
            )
          end
        end

        def render_role_field
          FormField(class: 'space-y-2') do
            FormFieldLabel(for: 'invitation_role',
                           class: 'text-[10px] font-black uppercase tracking-widest text-on-surface-variant ml-1') do
              t('admin.invitations.index.form.role')
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
            m3_button(type: :submit, variant: :filled, size: :lg,
                   class: 'px-8 rounded-2xl shadow-lg shadow-primary/20') do
              t('admin.invitations.index.form.submit')
            end
          end
        end

        def render_existing_invitations
          return if @invitations.empty?

          div(class: 'max-w-4xl mx-auto w-full') do
            m3_card(class: 'overflow-hidden border-none shadow-xl rounded-[2rem] bg-card') do
              div(class: 'px-8 py-6 border-b border-border') do
                m3_heading(level: 2, size: '5', class: 'font-bold tracking-tight') { t('admin.invitations.index.recent') }
              end

              div(class: 'divide-y divide-border') do
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
              p(class: 'text-sm font-semibold text-foreground') { invitation.email }
              p(class: 'text-sm text-on-surface-variant') do
                plain "#{invitation.role.titleize} • #{invitation_status_label(invitation)}"
              end
              p(class: 'text-xs text-on-surface-variant') { invitation_metadata(invitation) }
            end

            next unless resendable_invitation?(invitation)

            form_with(url: resend_admin_invitation_path(invitation), method: :post, class: 'shrink-0') do
              m3_button(type: :submit, variant: :outlinedd, size: :sm, class: 'rounded-xl') do
                t('admin.invitations.index.resend')
              end
            end
          end
        end

        def resendable_invitation?(invitation)
          @resendable_invitation_ids.include?(invitation.id)
        end

        def invitation_status_label(invitation)
          return t('admin.invitations.index.status.accepted') if invitation.accepted?
          return t('admin.invitations.index.status.expired') if invitation.expired?

          t('admin.invitations.index.status.pending')
        end

        def invitation_metadata(invitation)
          if invitation.accepted?
            t('admin.invitations.index.metadata.accepted', time: view_context.time_ago_in_words(invitation.accepted_at))
          elsif invitation.expired?
            t('admin.invitations.index.metadata.expired', time: view_context.time_ago_in_words(invitation.expires_at))
          else
            t('admin.invitations.index.metadata.expires', time: view_context.time_ago_in_words(invitation.expires_at))
          end
        end

        def render_errors
          render RubyUI::Alert.new(variant: :destructive, class: 'mb-6') do
            div do
              m3_heading(level: 2, size: '3', class: 'font-semibold mb-2') do
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
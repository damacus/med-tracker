# frozen_string_literal: true

module Components
  module Admin
    module Invitations
      class IndexView < Components::Base
        include Phlex::Rails::Helpers::FormWith
        include Phlex::Rails::Helpers::Pluralize
        include Components::FormHelpers

        def initialize(
          invitation: HouseholdInvitation.new,
          invitations: HouseholdInvitation.order(created_at: :desc),
          resendable_invitation_ids: [],
          cancellable_invitation_ids: [],
          dependents: Person.none
        )
          @invitation = invitation
          @invitations = invitations
          @resendable_invitation_ids = resendable_invitation_ids
          @cancellable_invitation_ids = cancellable_invitation_ids
          @dependents = dependents
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
          form_with(
            url: admin_invitations_path,
            method: :post,
            class: 'space-y-8',
            data: {
              controller: 'dependent-assignment',
              action: 'change->dependent-assignment#sync',
              dependent_assignment_roles_value: %w[parent carer family_member professional].to_json
            }
          ) do
            div(class: 'space-y-6') do
              render_email_field
              render_membership_role_field
              render_relationship_type_field
              render_access_level_field
              render_dependents_field
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
              class: 'rounded-shape-sm border-border bg-card py-4 px-4 focus:ring-2 ' \
                     'focus:ring-primary/10 focus:border-primary transition-all'
            )
          end
        end

        def render_membership_role_field
          FormField(class: 'space-y-2') do
            FormFieldLabel(for: 'invitation_membership_role',
                           class: 'text-[10px] font-black uppercase tracking-widest text-on-surface-variant ml-1') do
              t('admin.invitations.index.form.role')
            end
            m3_select(name: 'invitation[membership_role]', id: 'invitation_membership_role', size: :sm,
                      required: true) do
              HouseholdInvitation.membership_roles.each_key do |role|
                option(value: role, selected: selected_membership_role == role) { role.titleize }
              end
            end
          end
        end

        def render_relationship_type_field
          FormField(class: 'space-y-2') do
            FormFieldLabel(for: 'invitation_relationship_type',
                           class: 'text-[10px] font-black uppercase tracking-widest text-on-surface-variant ml-1') do
              t('admin.invitations.index.form.relationship_type', default: 'Dependent relationship')
            end
            m3_select(name: 'invitation[relationship_type]', id: 'invitation_relationship_type', size: :sm) do
              option(value: '', selected: selected_relationship_type.blank?) do
                t('admin.invitations.index.form.select_relationship_type', default: 'Select relationship')
              end
              %w[parent carer family_member professional].each do |relationship_type|
                option(value: relationship_type, selected: selected_relationship_type == relationship_type) do
                  relationship_type.titleize
                end
              end
            end
          end
        end

        def render_access_level_field
          FormField(class: 'space-y-2') do
            FormFieldLabel(for: 'invitation_access_level',
                           class: 'text-[10px] font-black uppercase tracking-widest text-on-surface-variant ml-1') do
              t('admin.invitations.index.form.access_level', default: 'Dependent access')
            end
            m3_select(name: 'invitation[access_level]', id: 'invitation_access_level', size: :sm) do
              HouseholdInvitationGrant.access_levels.each_key do |access_level|
                option(value: access_level, selected: selected_access_level == access_level) { access_level.titleize }
              end
            end
          end
        end

        def render_dependents_field
          return if @dependents.empty?

          FormField(
            class: 'space-y-3',
            hidden: !dependent_assignment_role?,
            data: { dependent_assignment_target: 'field' }
          ) do
            FormFieldLabel(for: 'invitation_dependent_ids',
                           class: 'text-[10px] font-black uppercase tracking-widest text-on-surface-variant ml-1') do
              t('admin.invitations.index.form.dependents')
            end
            p(class: 'text-sm text-on-surface-variant') { t('admin.invitations.index.form.dependents_hint') }
            div(class: 'grid grid-cols-1 sm:grid-cols-2 gap-3') do
              @dependents.each do |dependent|
                label(
                  class: 'flex items-center gap-3 p-4 rounded-xl border border-outline-variant ' \
                         'bg-surface-container-low hover:bg-surface-container-high cursor-pointer ' \
                         'transition-all state-layer relative'
                ) do
                  input(
                    type: 'checkbox',
                    name: 'invitation[dependent_ids][]',
                    value: dependent.id,
                    id: "invitation_dependent_#{dependent.id}",
                    checked: selected_dependent_ids.include?(dependent.id),
                    disabled: !dependent_assignment_role?,
                    class: "z-10 #{checkbox_classes}"
                  )
                  span(class: 'z-10 font-bold text-foreground') { dependent.name }
                end
              end
            end
            input(type: 'hidden', name: 'invitation[dependent_ids][]', value: '', disabled: !dependent_assignment_role?)
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
                m3_heading(level: 2, size: '5', class: 'font-bold tracking-tight') do
                  t('admin.invitations.index.recent')
                end
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
                plain "#{invitation.membership_role.titleize} • #{invitation_status_label(invitation)}"
              end
              p(class: 'text-xs text-on-surface-variant') { invitation_metadata(invitation) }
            end

            render_invitation_actions(invitation)
          end
        end

        def render_invitation_actions(invitation)
          return unless resendable_invitation?(invitation) || cancellable_invitation?(invitation)

          div(class: 'flex items-center gap-2 shrink-0') do
            if resendable_invitation?(invitation)
              form_with(url: resend_admin_invitation_path(invitation), method: :post, class: 'shrink-0') do
                m3_button(type: :submit, variant: :outlined, size: :sm, class: 'rounded-xl') do
                  t('admin.invitations.index.resend')
                end
              end
            end

            if cancellable_invitation?(invitation)
              form_with(url: admin_invitation_path(invitation), method: :delete, class: 'shrink-0') do
                m3_button(type: :submit, variant: :outlined, size: :sm, class: 'rounded-xl') do
                  t('admin.invitations.index.cancel')
                end
              end
            end
          end
        end

        def resendable_invitation?(invitation)
          @resendable_invitation_ids.include?(invitation.id)
        end

        def cancellable_invitation?(invitation)
          @cancellable_invitation_ids.include?(invitation.id)
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

        def selected_dependent_ids
          @selected_dependent_ids ||=
            Array(@invitation.dependent_ids).presence&.map(&:to_i) ||
            @invitation.household_invitation_grants.map(&:person_id)
        end

        def dependent_assignment_role?
          %w[parent carer family_member professional].include?(selected_relationship_type)
        end

        def selected_membership_role
          @invitation.membership_role.presence || 'member'
        end

        def selected_relationship_type
          @invitation.relationship_type.to_s
        end

        def selected_access_level
          @invitation.access_level.presence || 'record'
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

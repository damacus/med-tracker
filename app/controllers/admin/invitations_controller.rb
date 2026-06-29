# frozen_string_literal: true

module Admin
  class InvitationsController < BaseController
    def index
      authorize :invitation, :index?

      render invitation_index_view
    end

    def create
      authorize :invitation, :create?

      @invitation = build_household_invitation

      respond_to do |format|
        if create_household_invitation
          InvitationMailer.with(invitation: @invitation, token: @invitation.plain_token).invite.deliver_later
          format.html { redirect_to admin_invitations_path, notice: t('admin.invitations.created') }
          format.turbo_stream do
            flash.now[:notice] = t('admin.invitations.created')
            render_index_turbo
          end
        else
          format.html do
            render invitation_index_view(invitation: @invitation), status: :unprocessable_content
          end
          format.turbo_stream do
            render turbo_stream: turbo_stream.replace(
              'admin_invitations',
              invitation_index_view(invitation: @invitation)
            ), status: :unprocessable_content
          end
        end
      end
    end

    def resend
      @invitation = current_household.household_invitations.find(params.expect(:id))
      authorize @invitation, :resend?

      if @invitation.resendable?
        @invitation.resend!
        InvitationMailer.with(invitation: @invitation, token: @invitation.plain_token).invite.deliver_later
        redirect_with_invitation_notice(t('admin.invitations.resent'))
      elsif @invitation.accepted?
        redirect_with_invitation_alert(t('admin.invitations.cannot_resend_accepted'))
      else
        redirect_with_invitation_alert(t('admin.invitations.resend_failed'))
      end
    end

    def destroy
      @invitation = current_household.household_invitations.find(params.expect(:id))
      authorize @invitation, :destroy?

      if @invitation.cancellable?
        @invitation.destroy!
        redirect_with_invitation_notice(t('admin.invitations.cancelled'))
      else
        redirect_with_invitation_alert(t('admin.invitations.cannot_cancel_accepted'))
      end
    end

    private

    def invitation_params
      params.expect(invitation: [:email, :membership_role, :access_level, :relationship_type, { dependent_ids: [] }])
    end

    def build_household_invitation
      invitation = current_household.household_invitations.new(
        email: invitation_params[:email],
        membership_role: invitation_params[:membership_role],
        invited_by_membership: current_membership
      )
      invitation.access_level = invitation_params[:access_level]
      invitation.relationship_type = invitation_params[:relationship_type]
      invitation.dependent_ids = invitation_params[:dependent_ids]
      invitation
    end

    def create_household_invitation
      ActiveRecord::Base.transaction do
        @invitation.save!
        create_invitation_grants!
      end
      true
    rescue ActiveRecord::RecordInvalid => e
      @invitation = e.record if e.record.is_a?(HouseholdInvitation)
      false
    end

    def create_invitation_grants!
      selected_dependents.find_each do |dependent|
        @invitation.household_invitation_grants.create!(
          household: current_household,
          person: dependent,
          access_level: invitation_access_level,
          relationship_type: invitation_relationship_type
        )
      end
    end

    def selected_dependents
      current_household.people.where(id: selected_dependent_ids, person_type: %i[minor dependent_adult], has_capacity: false)
    end

    def selected_dependent_ids
      Array(invitation_params[:dependent_ids]).filter_map do |id|
        id.to_i if id.to_s.match?(/\A\d+\z/)
      end.uniq
    end

    def invitation_access_level
      access_level = invitation_params[:access_level].presence
      return access_level if HouseholdInvitationGrant.access_levels.key?(access_level)

      invitation_relationship_type == 'parent' ? :manage : :record
    end

    def invitation_relationship_type
      relationship_type = invitation_params[:relationship_type].presence
      if HouseholdInvitationGrant.relationship_types.key?(relationship_type) && relationship_type != 'self'
        return relationship_type
      end

      :professional
    end

    def render_index_turbo
      render turbo_stream: [
        turbo_stream.replace('admin_invitations', invitation_index_view),
        turbo_stream.update('flash', Components::Layouts::Flash.new(notice: flash[:notice], alert: flash[:alert]))
      ]
    end

    def invitation_index_view(invitation: HouseholdInvitation.new(membership_role: :member, access_level: :record))
      result = Admin::InvitationsIndexQuery.new(scope: current_household.household_invitations).call
      Components::Admin::Invitations::IndexView.new(
        invitation: invitation,
        invitations: result.invitations,
        resendable_invitation_ids: result.resendable_invitation_ids,
        cancellable_invitation_ids: result.cancellable_invitation_ids,
        dependents: load_dependents
      )
    end

    def load_dependents
      @load_dependents ||=
        current_household.people.where(person_type: %i[minor dependent_adult], has_capacity: false).order(:name).to_a
    end

    def redirect_with_invitation_notice(message)
      respond_to do |format|
        format.html { redirect_to admin_invitations_path, notice: message }
        format.turbo_stream do
          flash.now[:notice] = message
          render_index_turbo
        end
      end
    end

    def redirect_with_invitation_alert(message)
      respond_to do |format|
        format.html { redirect_to admin_invitations_path, alert: message }
        format.turbo_stream do
          flash.now[:alert] = message
          render_index_turbo
        end
      end
    end
  end
end

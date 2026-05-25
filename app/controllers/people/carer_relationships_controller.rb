# frozen_string_literal: true

module People
  class CarerRelationshipsController < ApplicationController
    before_action :set_patient

    def new
      @relationship = CarerRelationship.new(patient: @patient)
      authorize @relationship, :assign_dependent?

      render_form
    end

    def create
      @relationship = CarerRelationship.new(patient: @patient)
      authorize @relationship, :assign_dependent?

      if current_user.administrator?
        create_admin_assignment
      else
        create_parent_assignment
      end
    end

    private

    def set_patient
      @patient = Person.find(params.expect(:person_id))
    end

    def create_admin_assignment
      carer = assignable_carers.find_by(id: admin_assignment_params[:carer_id])
      unless carer
        render_invalid_assignment(t('people.carer_relationships.missing_carer'))
        return
      end

      DependentRelationshipAssigner.new(
        carer: carer,
        dependent_ids: [@patient.id],
        relationship_type: admin_assignment_params[:relationship_type]
      ).call
      redirect_to @patient, notice: t('people.carer_relationships.created')
    end

    def create_parent_assignment
      user = parent_user_by_email

      if assignable_parent_user?(user)
        assign_existing_user(user)
      elsif user
        render_invalid_assignment(t('people.carer_relationships.invalid_role'))
      else
        invite_parent
      end
    end

    def assign_existing_user(user)
      DependentRelationshipAssigner.new(
        carer: user.person,
        dependent_ids: [@patient.id],
        relationship_type: DependentRelationshipAssigner.relationship_type_for(user)
      ).call
      redirect_to @patient, notice: t('people.carer_relationships.created')
    end

    def invite_parent
      invitation = pending_invitation_for_parent_email || Invitation.new(email: parent_assignment_email)
      invitation.role = :parent if invitation.new_record?

      unless invitation.parent?
        render_invalid_assignment(t('people.carer_relationships.invalid_role'))
        return
      end

      invitation.dependents << @patient unless invitation.dependents.include?(@patient)

      if invitation.save
        deliver_invitation(invitation) if invitation.previous_changes.key?('id')
        redirect_to @patient, notice: t('people.carer_relationships.invited')
      else
        @relationship.errors.merge!(invitation.errors)
        render_form(status: :unprocessable_content)
      end
    end

    def deliver_invitation(invitation)
      InvitationMailer.with(invitation: invitation, token: invitation.plain_token).invite.deliver_later
    end

    def render_invalid_assignment(message)
      @relationship.errors.add(:base, message)
      render_form(status: :unprocessable_content)
    end

    def render_form(status: :ok)
      render Components::People::CarerRelationships::FormView.new(
        relationship: @relationship,
        patient: @patient,
        carers: assignable_carers,
        current_user: current_user,
        email: parent_assignment_email
      ), status: status
    end

    def assignable_carers
      return Person.none unless current_user.administrator?

      Person.joins(:user).where(person_type: :adult, has_capacity: true).order(:name)
    end

    def assignable_parent_user?(user)
      user&.parent? && user.person&.person_type == 'adult' && user.person&.has_capacity?
    end

    def parent_user_by_email
      User.active.find_by('LOWER(email_address) = ?', parent_assignment_email)
    end

    def pending_invitation_for_parent_email
      Invitation.pending.find_by('LOWER(email) = ?', parent_assignment_email)
    end

    def admin_assignment_params
      params.expect(carer_relationship: %i[carer_id relationship_type])
    end

    def parent_assignment_email
      params.dig(:carer_relationship, :email).to_s.strip.downcase
    end
  end
end

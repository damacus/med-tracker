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

      if household_manager?
        create_admin_assignment
      else
        create_parent_assignment
      end
    rescue CareDelegation::Assign::Error => e
      render_invalid_assignment(e.message)
    end

    private

    def set_patient
      @patient = policy_scope(Person).find(params.expect(:person_id))
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
        relationship_type: admin_assignment_params[:relationship_type],
        scope: @patient.household.people,
        granted_by_membership: current_membership
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
        relationship_type: 'parent',
        scope: @patient.household.people,
        granted_by_membership: current_membership
      ).call
      redirect_to @patient, notice: t('people.carer_relationships.created')
    end

    def invite_parent
      invitation = pending_invitation_for_parent_email || current_household.household_invitations.new(
        email: parent_assignment_email,
        membership_role: :member,
        invited_by_membership: current_membership
      )
      invitation.relationship_type = :parent
      invitation.access_level = :manage

      unless parent_invitation?(invitation)
        render_invalid_assignment(t('people.carer_relationships.invalid_role'))
        return
      end

      was_new_record = invitation.new_record?
      if save_parent_invitation(invitation)
        deliver_invitation(invitation) if was_new_record
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
        options: {
          household_manager: household_manager?,
          email: parent_assignment_email
        }
      ), status: status
    end

    def assignable_carers
      return Person.none unless household_manager?

      policy_scope(Person).joins(:user).where(person_type: :adult, has_capacity: true).order(:name)
    end

    def assignable_parent_user?(user)
      user&.person&.person_type == 'adult' &&
        user.person&.has_capacity? &&
        user.person.professional_title.blank? &&
        !self_managing_adult?(user.person)
    end

    def self_managing_adult?(person)
      person.adult? &&
        person.has_capacity? &&
        CarerRelationship.active.exists?(carer: person, patient: person, relationship_type: %w[self 0])
    end

    def parent_invitation?(invitation)
      invitation.new_record? ||
        invitation.household_invitation_grants.empty? ||
        invitation.household_invitation_grants.any? { |grant| grant.relationship_type == 'parent' }
    end

    def save_parent_invitation(invitation)
      ActiveRecord::Base.transaction do
        invitation.save!
        invitation.household_invitation_grants.find_or_create_by!(household: current_household, person: @patient) do |grant|
          grant.access_level = :manage
          grant.relationship_type = :parent
        end
      end
      true
    rescue ActiveRecord::RecordInvalid => e
      invitation.errors.merge!(e.record.errors) unless e.record == invitation
      false
    end

    def parent_user_by_email
      User.active.find_by('LOWER(email_address) = ?', parent_assignment_email)
    end

    def pending_invitation_for_parent_email
      current_household.household_invitations.pending.find_by('LOWER(email) = ?', parent_assignment_email)
    end

    def admin_assignment_params
      params.expect(carer_relationship: %i[carer_id relationship_type])
    end

    def parent_assignment_email
      params.dig(:carer_relationship, :email).to_s.strip.downcase
    end

    def household_manager?
      current_membership&.owner? || current_membership&.administrator?
    end
  end
end

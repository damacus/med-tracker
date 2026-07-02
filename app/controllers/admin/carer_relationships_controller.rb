# frozen_string_literal: true

module Admin
  # Handles admin management of carer-patient relationships.
  class CarerRelationshipsController < BaseController
    def index
      authorize CarerRelationship
      relationships = Admin::CarerRelationshipsIndexQuery.new(scope: policy_scope(CarerRelationship)).call
      @pagy, relationships = pagy(:offset, relationships)
      render Components::Admin::CarerRelationships::IndexView.new(
        relationships: relationships,
        current_user: current_user,
        pagy: @pagy
      )
    end

    def new
      @relationship = CarerRelationship.new(patient_id: params[:patient_id])
      authorize @relationship
      options = carer_relationship_options

      render_modal_or_page(
        modal: lambda {
          Components::Admin::CarerRelationships::FormView.new(
            relationship: @relationship,
            carers: options.carers,
            patients: options.patients,
            modal: true
          )
        },
        page: lambda {
          Components::Admin::CarerRelationships::FormView.new(
            relationship: @relationship,
            carers: options.carers,
            patients: options.patients
          )
        }
      )
    end

    def create
      @relationship = CarerRelationship.new(relationship_params)
      authorize @relationship
      options = carer_relationship_options

      respond_to do |format|
        if @relationship.save
          grant_relationship_access(@relationship)
          format.html { redirect_to admin_carer_relationships_path, notice: t('admin.carer_relationships.created') }
          format.turbo_stream do
            flash.now[:notice] = t('admin.carer_relationships.created')
            render turbo_stream: [
              turbo_stream.update('modal', ''),
              turbo_stream.remove('carer_relationships_empty'),
              turbo_stream.prepend(
                'carer_relationships_rows',
                Components::Admin::CarerRelationships::Row.new(relationship: @relationship.reload)
              ),
              turbo_stream.update('flash', Components::Layouts::Flash.new(notice: flash[:notice], alert: flash[:alert]))
            ]
          end
        else
          format.html do
            render Components::Admin::CarerRelationships::FormView.new(
              relationship: @relationship,
              carers: options.carers,
              patients: options.patients
            ), status: :unprocessable_content
          end
          format.turbo_stream do
            render turbo_stream: turbo_stream.replace(
              'modal',
              Components::Admin::CarerRelationships::FormView.new(
                relationship: @relationship,
                carers: options.carers,
                patients: options.patients,
                modal: true
              )
            ), status: :unprocessable_content
          end
        end
      end
    end

    def destroy
      @relationship = policy_scope(CarerRelationship).find(params.expect(:id))
      authorize @relationship

      @relationship.deactivate!
      revoke_relationship_access(@relationship)
      respond_to do |format|
        format.html { redirect_to admin_carer_relationships_path, notice: t('admin.carer_relationships.deactivated') }
        format.turbo_stream do
          flash.now[:notice] = t('admin.carer_relationships.deactivated')
          render turbo_stream: relationship_row_streams(@relationship)
        end
      end
    end

    def activate
      @relationship = policy_scope(CarerRelationship).find(params.expect(:id))
      authorize @relationship

      @relationship.activate!
      grant_relationship_access(@relationship)
      respond_to do |format|
        format.html { redirect_to admin_carer_relationships_path, notice: t('admin.carer_relationships.activated') }
        format.turbo_stream do
          flash.now[:notice] = t('admin.carer_relationships.activated')
          render turbo_stream: relationship_row_streams(@relationship)
        end
      end
    end

    private

    def relationship_params
      params.expect(carer_relationship: %i[carer_id patient_id relationship_type])
    end

    def carer_relationship_options
      @carer_relationship_options ||= Admin::CarerRelationshipOptionsQuery.new(scope: policy_scope(Person)).call
    end

    def relationship_row_streams(relationship)
      [
        turbo_stream.replace(
          "carer_relationship_#{relationship.id}",
          Components::Admin::CarerRelationships::Row.new(relationship: relationship.reload)
        ),
        turbo_stream.update('flash', Components::Layouts::Flash.new(notice: flash[:notice], alert: flash[:alert]))
      ]
    end

    def grant_relationship_access(relationship)
      household = admin_household
      return unless household && relationship.carer.account

      relationship.carer.update!(household: household) if relationship.carer.household_id.blank?
      return unless relationship.carer.household_id == household.id

      membership = household.household_memberships.find_or_initialize_by(account: relationship.carer.account)
      membership.person = relationship.carer
      membership.role ||= :member
      membership.status = :active
      membership.save!

      grant_access(household, membership, relationship.carer, :manage, :self)
      grant_access(
        household,
        membership,
        relationship.patient,
        access_level_for(relationship.relationship_type),
        grant_relationship_type_for(relationship.relationship_type)
      )
    end

    def revoke_relationship_access(relationship)
      household = admin_household
      return unless household && relationship.carer.account

      membership = household.household_memberships.find_by(account: relationship.carer.account)
      return unless membership

      household.person_access_grants.active.find_by(
        household_membership: membership,
        person: relationship.patient
      )&.update!(revoked_at: Time.current)
    end

    def grant_access(household, membership, person, access_level, relationship_type)
      grant = household.person_access_grants.find_or_initialize_by(
        household_membership: membership,
        person: person
      )
      grant.access_level = access_level
      grant.relationship_type = relationship_type
      grant.granted_by_membership = current_membership || admin_membership_for(household) || membership
      grant.revoked_at = nil
      grant.save!
    end

    def admin_household
      current_household || current_account&.first_active_household
    end

    def admin_membership_for(household)
      current_membership || current_account&.active_household_membership_for(household)
    end

    def access_level_for(relationship_type)
      relationship_type == 'professional_carer' ? :record : :manage
    end

    def grant_relationship_type_for(relationship_type)
      return :professional if relationship_type == 'professional_carer'
      return :parent if relationship_type == 'parent'

      :family_member
    end
  end
end

# frozen_string_literal: true

module Admin
  # Handles admin management of carer-patient relationships.
  class CarerRelationshipsController < BaseController
    def index
      authorize CarerRelationship
      render_index
    end

    def render_index(status: :ok)
      relationships = Admin::CarerRelationshipsIndexQuery.new(scope: policy_scope(CarerRelationship)).call
      @pagy, relationships = pagy(:offset, relationships)
      render Components::Admin::CarerRelationships::IndexView.new(
        relationships: relationships,
        current_user: current_user,
        pagy: @pagy
      ), status: status
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
        if assign_relationship
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

      CareDelegation::Revoke.new(
        relationship: @relationship,
        actor_membership: relationship_actor_membership(@relationship)
      ).call
      respond_to do |format|
        format.html { redirect_to admin_carer_relationships_path, notice: t('admin.carer_relationships.deactivated') }
        format.turbo_stream do
          flash.now[:notice] = t('admin.carer_relationships.deactivated')
          render turbo_stream: relationship_row_streams(@relationship)
        end
      end
    rescue CareDelegation::Revoke::Error => e
      render_delegation_error(e.message)
    end

    def activate
      @relationship = policy_scope(CarerRelationship).find(params.expect(:id))
      authorize @relationship

      @relationship = CareDelegation::Assign.new(
        carer: @relationship.carer,
        patient: @relationship.patient,
        relationship_type: @relationship.relationship_type,
        granted_by_membership: relationship_actor_membership(@relationship)
      ).call
      respond_to do |format|
        format.html { redirect_to admin_carer_relationships_path, notice: t('admin.carer_relationships.activated') }
        format.turbo_stream do
          flash.now[:notice] = t('admin.carer_relationships.activated')
          render turbo_stream: relationship_row_streams(@relationship)
        end
      end
    rescue CareDelegation::Assign::Error => e
      render_delegation_error(e.message)
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

    def assign_relationship
      @relationship = CareDelegation::Assign.new(
        carer: @relationship.carer,
        patient: @relationship.patient,
        relationship_type: @relationship.relationship_type,
        granted_by_membership: relationship_actor_membership(@relationship)
      ).call
      true
    rescue ActiveRecord::RecordInvalid => e
      @relationship = e.record if e.record.is_a?(CarerRelationship)
      @relationship.errors.add(:base, e.message) if @relationship.errors.empty?
      false
    rescue CareDelegation::Assign::Error => e
      @relationship.errors.add(:base, e.message)
      false
    end

    def render_delegation_error(message)
      flash.now[:alert] = message
      respond_to do |format|
        format.html { render_index(status: :unprocessable_content) }
        format.turbo_stream do
          render turbo_stream: relationship_row_streams(@relationship), status: :unprocessable_content
        end
      end
    end

    def relationship_actor_membership(relationship)
      current_membership || current_account&.active_household_membership_for(relationship.household)
    end
  end
end

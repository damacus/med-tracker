# frozen_string_literal: true

module Admin
  # Handles admin management of carer-patient relationships.
  class CarerRelationshipsController < ApplicationController
    def index
      authorize CarerRelationship
      relationships = policy_scope(CarerRelationship).includes(:carer, :patient)
      relationships = relationships.order(created_at: :desc)
      @pagy, relationships = pagy(:offset, relationships)
      render Components::Admin::CarerRelationships::IndexView.new(
        relationships: relationships,
        current_user: current_user,
        pagy: @pagy
      )
    end

    def new
      @relationship = CarerRelationship.new
      authorize @relationship
      render Components::Admin::CarerRelationships::FormView.new(
        relationship: @relationship,
        carers: available_carers,
        patients: available_patients,
        url_helpers: self
      )
    end

    def create
      @relationship = CarerRelationship.new(relationship_params)
      authorize @relationship

      if @relationship.save
        redirect_to admin_carer_relationships_path, notice: 'Carer relationship was successfully created.'
      else
        render Components::Admin::CarerRelationships::FormView.new(
          relationship: @relationship,
          carers: available_carers,
          patients: available_patients,
          url_helpers: self
        ), status: :unprocessable_content
      end
    end

    def destroy
      @relationship = CarerRelationship.find(params[:id])
      authorize @relationship

      @relationship.deactivate!
      redirect_to admin_carer_relationships_path, notice: 'Carer relationship has been deactivated.'
    end

    def activate
      @relationship = CarerRelationship.find(params[:id])
      authorize @relationship

      @relationship.activate!
      redirect_to admin_carer_relationships_path, notice: 'Carer relationship has been activated.'
    end

    private

    def relationship_params
      params.expect(carer_relationship: %i[carer_id patient_id relationship_type])
    end

    def available_carers
      Person.joins(:user).where.not(users: { role: :minor }).order(:name)
    end

    def available_patients
      Person.order(:name)
    end
  end
end

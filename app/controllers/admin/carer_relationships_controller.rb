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
      @relationship = CarerRelationship.new(patient_id: params[:patient_id])
      authorize @relationship
      is_modal = request.headers['Turbo-Frame'] == 'modal'

      respond_to do |format|
        format.html do
          render Components::Admin::CarerRelationships::FormView.new(
            relationship: @relationship,
            carers: available_carers,
            patients: available_patients,
            modal: is_modal
          ), layout: false
        end
        format.turbo_stream do
          render turbo_stream: turbo_stream.replace(
            'modal',
            Components::Admin::CarerRelationships::FormView.new(
              relationship: @relationship,
              carers: available_carers,
              patients: available_patients,
              modal: true
            )
          )
        end
      end
    end

    def create
      @relationship = CarerRelationship.new(relationship_params)
      authorize @relationship

      respond_to do |format|
        if @relationship.save
          format.html { redirect_to admin_carer_relationships_path, notice: 'Carer relationship was successfully created.' }
          format.turbo_stream do
            flash.now[:notice] = 'Carer relationship was successfully created.'
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
              carers: available_carers,
              patients: available_patients
            ), status: :unprocessable_content
          end
          format.turbo_stream do
            render turbo_stream: turbo_stream.replace(
              'modal',
              Components::Admin::CarerRelationships::FormView.new(
                relationship: @relationship,
                carers: available_carers,
                patients: available_patients,
                modal: true
              )
            ), status: :unprocessable_content
          end
        end
      end
    end

    def destroy
      @relationship = CarerRelationship.find(params[:id])
      authorize @relationship

      @relationship.deactivate!
      respond_to do |format|
        format.html { redirect_to admin_carer_relationships_path, notice: 'Carer relationship has been deactivated.' }
        format.turbo_stream do
          flash.now[:notice] = 'Carer relationship has been deactivated.'
          render turbo_stream: relationship_row_streams(@relationship)
        end
      end
    end

    def activate
      @relationship = CarerRelationship.find(params[:id])
      authorize @relationship

      @relationship.activate!
      respond_to do |format|
        format.html { redirect_to admin_carer_relationships_path, notice: 'Carer relationship has been activated.' }
        format.turbo_stream do
          flash.now[:notice] = 'Carer relationship has been activated.'
          render turbo_stream: relationship_row_streams(@relationship)
        end
      end
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

    def relationship_row_streams(relationship)
      [
        turbo_stream.replace(
          "carer_relationship_#{relationship.id}",
          Components::Admin::CarerRelationships::Row.new(relationship: relationship.reload)
        ),
        turbo_stream.update('flash', Components::Layouts::Flash.new(notice: flash[:notice], alert: flash[:alert]))
      ]
    end
  end
end

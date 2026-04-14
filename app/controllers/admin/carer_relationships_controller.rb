# frozen_string_literal: true

module Admin
  # Handles admin management of carer-patient relationships.
  class CarerRelationshipsController < ApplicationController
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
      is_modal = request.headers['Turbo-Frame'] == 'modal'
      options = carer_relationship_options

      respond_to do |format|
        format.html do
          render Components::Admin::CarerRelationships::FormView.new(
            relationship: @relationship,
            carers: options.carers,
            patients: options.patients,
            modal: is_modal
          ), layout: false
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
          )
        end
      end
    end

    def create
      @relationship = CarerRelationship.new(relationship_params)
      authorize @relationship
      options = carer_relationship_options

      respond_to do |format|
        if @relationship.save
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
      @relationship = policy_scope(CarerRelationship).find(params[:id])
      authorize @relationship

      @relationship.deactivate!
      respond_to do |format|
        format.html { redirect_to admin_carer_relationships_path, notice: t('admin.carer_relationships.deactivated') }
        format.turbo_stream do
          flash.now[:notice] = t('admin.carer_relationships.deactivated')
          render turbo_stream: relationship_row_streams(@relationship)
        end
      end
    end

    def activate
      @relationship = policy_scope(CarerRelationship).find(params[:id])
      authorize @relationship

      @relationship.activate!
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
  end
end

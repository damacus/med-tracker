# frozen_string_literal: true

class LocationMembershipsController < ApplicationController
  before_action :set_location

  def create
    authorize LocationMembership
    @person = policy_scope(Person).find(params[:location_membership][:person_id])

    if LocationMembership.create(location: @location, person: @person)
      respond_to do |format|
        format.html { redirect_to @location, notice: t('.success', name: @person.name, location: @location.name) }
        format.turbo_stream do
          flash.now[:notice] = t('.success', name: @person.name, location: @location.name)
          render turbo_stream: location_show_streams
        end
      end
    else
      respond_to do |format|
        format.html { redirect_to @location, alert: t('.failure') }
        format.turbo_stream do
          flash.now[:alert] = t('.failure')
          render turbo_stream: location_show_streams, status: :unprocessable_content
        end
      end
    end
  end

  def destroy
    @membership = @location.location_memberships.find(params[:id])
    authorize @membership
    @person = @membership.person

    if @membership.destroy
      respond_to do |format|
        format.html { redirect_to @location, notice: t('.success', name: @person.name, location: @location.name) }
        format.turbo_stream do
          flash.now[:notice] = t('.success', name: @person.name, location: @location.name)
          render turbo_stream: location_show_streams
        end
      end
    else
      respond_to do |format|
        format.html { redirect_to @location, alert: t('.failure') }
        format.turbo_stream do
          flash.now[:alert] = t('.failure')
          render turbo_stream: location_show_streams, status: :unprocessable_content
        end
      end
    end
  end

  private

  def set_location
    @location = policy_scope(Location).find(params[:location_id])
  end

  def location_show_streams
    [
      turbo_stream.replace("location_show_#{@location.id}", Components::Locations::ShowView.new(location: @location.reload)),
      turbo_stream.update('flash', Components::Layouts::Flash.new(notice: flash[:notice], alert: flash[:alert]))
    ]
  end
end

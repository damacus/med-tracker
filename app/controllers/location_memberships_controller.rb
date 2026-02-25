# frozen_string_literal: true

class LocationMembershipsController < ApplicationController
  before_action :set_location

  def create
    authorize LocationMembership
    @person = Person.find(params[:location_membership][:person_id])
    
    if LocationMembership.create(location: @location, person: @person)
      redirect_to @location, notice: t('location_memberships.create.success', name: @person.name, location: @location.name)
    else
      redirect_to @location, alert: t('location_memberships.create.failure')
    end
  end

  def destroy
    @membership = @location.location_memberships.find(params[:id])
    authorize @membership
    @person = @membership.person
    
    if @membership.destroy
      redirect_to @location, notice: t('location_memberships.destroy.success', name: @person.name, location: @location.name)
    else
      redirect_to @location, alert: t('location_memberships.destroy.failure')
    end
  end

  private

  def set_location
    @location = Location.find(params[:location_id])
  end
end

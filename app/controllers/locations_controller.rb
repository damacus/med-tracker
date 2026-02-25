# frozen_string_literal: true

class LocationsController < ApplicationController
  before_action :set_location, only: %i[show edit update destroy]

  def index
    locations = policy_scope(Location).includes(:medicines, :members)
    render Components::Locations::IndexView.new(locations: locations)
  end

  def show
    authorize @location
    render Components::Locations::ShowView.new(location: @location, notice: flash[:notice])
  end

  def new
    @location = Location.new
    authorize @location
    render Components::Locations::FormView.new(
      location: @location,
      title: 'New Location',
      subtitle: 'Add a new medicine storage location'
    )
  end

  def edit
    authorize @location
    render Components::Locations::FormView.new(
      location: @location,
      title: 'Edit Location',
      subtitle: @location.name
    )
  end

  def create
    @location = Location.new(location_params)
    authorize @location

    if @location.save
      redirect_to @location, notice: t('locations.created')
    else
      render Components::Locations::FormView.new(
        location: @location,
        title: 'New Location',
        subtitle: 'Add a new medicine storage location'
      ), status: :unprocessable_content
    end
  end

  def update
    authorize @location
    if @location.update(location_params)
      redirect_back_or_to @location, notice: t('locations.updated')
    else
      render Components::Locations::FormView.new(
        location: @location,
        title: 'Edit Location',
        subtitle: @location.name
      ), status: :unprocessable_content
    end
  end

  def destroy
    authorize @location
    @location.destroy
    redirect_to locations_url, notice: t('locations.deleted')
  end

  private

  def set_location
    @location = policy_scope(Location).includes(:medicines, :members).find(params[:id])
  end

  def location_params
    params.expect(location: %i[name description])
  end
end

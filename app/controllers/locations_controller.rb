# frozen_string_literal: true

class LocationsController < ApplicationController
  before_action :set_location, only: %i[show edit update destroy]

  def index
    authorize Location
    render locations_index_view
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
      subtitle: 'Add a new medication storage location'
    )
  end

  def edit
    authorize @location
    @return_to = url_from(params[:return_to])
    render Components::Locations::FormView.new(
      location: @location,
      title: 'Edit Location',
      subtitle: @location.name,
      return_to: @return_to
    )
  end

  def create
    @location = Location.new(location_params)
    authorize @location

    if @location.save
      respond_to do |format|
        format.html { redirect_to @location, notice: t('locations.created') }
        format.turbo_stream do
          flash.now[:notice] = t('locations.created')
          render turbo_stream: location_main_content_streams(@location.reload)
        end
      end
    else
      respond_to do |format|
        format.html { render new_location_form, status: :unprocessable_content }
        format.turbo_stream do
          render turbo_stream: turbo_stream.replace('main-content', new_location_form),
                 status: :unprocessable_content
        end
      end
    end
  end

  def update
    authorize @location
    if @location.update(location_params)
      respond_to do |format|
        format.html { redirect_to safe_redirect_path(params[:return_to]) || @location, notice: t('locations.updated') }
        format.turbo_stream do
          flash.now[:notice] = t('locations.updated')
          render turbo_stream: location_main_content_streams(@location.reload)
        end
      end
    else
      respond_to do |format|
        format.html { render edit_location_form, status: :unprocessable_content }
        format.turbo_stream do
          render turbo_stream: turbo_stream.replace('main-content', edit_location_form),
                 status: :unprocessable_content
        end
      end
    end
  end

  def destroy
    authorize @location
    location_id = @location.id
    @location.destroy
    respond_to do |format|
      format.html { redirect_to locations_url, notice: t('locations.deleted') }
      format.turbo_stream do
        flash.now[:notice] = t('locations.deleted')
        render turbo_stream: [
          turbo_stream.remove("location_#{location_id}"),
          turbo_stream.remove("location_show_#{location_id}"),
          turbo_stream.update('flash', Components::Layouts::Flash.new(notice: flash[:notice], alert: flash[:alert]))
        ]
      end
    end
  end

  private

  def set_location
    @location = locations_query.find(id: params.expect(:id))
  end

  def location_params
    params.expect(location: %i[name description])
  end

  def locations_query
    LocationsQuery.new(scope: policy_scope(Location))
  end

  def locations_index_view
    Components::Locations::IndexView.new(locations: locations_query.index)
  end

  def new_location_form
    Components::Locations::FormView.new(
      location: @location,
      title: 'New Location',
      subtitle: 'Add a new medication storage location'
    )
  end

  def edit_location_form
    Components::Locations::FormView.new(
      location: @location,
      title: 'Edit Location',
      subtitle: @location.name,
      return_to: url_from(params[:return_to])
    )
  end

  def location_main_content_streams(location)
    [
      turbo_stream.replace('main-content', Components::Locations::ShowView.new(location: location)),
      turbo_stream.update('flash', Components::Layouts::Flash.new(notice: flash[:notice], alert: flash[:alert]))
    ]
  end
end

# frozen_string_literal: true

class LocationsController < ApplicationController
  before_action :set_location, only: %i[show edit update destroy]

  def index
    render locations_index_view
  end

  def show
    authorize @location
    render Components::Locations::ShowView.new(
      location: @location,
      notice: flash[:notice],
      available_people: available_people_for_location(@location)
    )
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
        format.turbo_stream { render_create_stream }
      end
    else
      render_error(new_location_form)
    end
  end

  def update
    authorize @location
    if @location.update(location_params)
      respond_to do |format|
        format.html { redirect_to safe_redirect_path(params[:return_to]) || @location, notice: t('locations.updated') }
        format.turbo_stream { render_update_stream }
      end
    else
      render_error(edit_location_form)
    end
  end

  def destroy
    authorize @location
    destroy_location
  end

  private

  def destroy_location
    if MedicationAdministrationHistory.exists_for?(@location)
      @location.errors.add(:base, 'Location cannot be deleted while administration history exists')
      return render_destroy_failure
    end

    location_id = @location.id
    return render_destroy_failure unless @location.destroy

    respond_to do |format|
      format.html { redirect_to locations_url, notice: t('locations.deleted') }
      format.turbo_stream { render_destroy_stream(location_id) }
    end
  end

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
      turbo_stream.replace(
        'main-content',
        Components::Locations::ShowView.new(
          location: location,
          available_people: available_people_for_location(location)
        )
      ),
      turbo_stream.update('flash', Components::Layouts::Flash.new(notice: flash[:notice], alert: flash[:alert]))
    ]
  end

  def available_people_for_location(location)
    policy_scope(Person).where.not(id: location.member_ids).order(:name)
  end

  def render_create_stream
    flash.now[:notice] = t('locations.created')
    render turbo_stream: location_main_content_streams(@location.reload)
  end

  def render_update_stream
    flash.now[:notice] = t('locations.updated')
    render turbo_stream: location_main_content_streams(@location.reload)
  end

  def render_destroy_stream(location_id)
    flash.now[:notice] = t('locations.deleted')
    render turbo_stream: [
      turbo_stream.remove(tenant_dom_target("location_#{location_id}")),
      turbo_stream.remove(tenant_dom_target("location_show_#{location_id}")),
      turbo_stream.update('flash', Components::Layouts::Flash.new(notice: flash[:notice], alert: flash[:alert]))
    ]
  end

  def render_destroy_failure
    message = @location.errors.full_messages.to_sentence.presence || 'Location could not be deleted'
    respond_to do |format|
      format.html { redirect_to @location, alert: message, status: :see_other }
      format.turbo_stream do
        flash.now[:alert] = message
        render turbo_stream: turbo_stream.update(
          'flash',
          Components::Layouts::Flash.new(notice: flash[:notice], alert: flash[:alert])
        ), status: :unprocessable_content
      end
    end
  end

  def render_error(form)
    respond_to do |format|
      format.html { render form, status: :unprocessable_content }
      format.turbo_stream { render_error_stream(form) }
    end
  end

  def render_error_stream(form)
    render turbo_stream: turbo_stream.replace('main-content', form), status: :unprocessable_content
  end
end

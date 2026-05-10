# frozen_string_literal: true

module InventoryLocationFilterable
  extend ActiveSupport::Concern

  private

  def accessible_inventory_locations(medications)
    InventoryLocationsQuery.new(medications_scope: medications).call
  end

  def resolved_inventory_location_id(locations)
    location_ids = locations.map(&:id)
    return remembered_inventory_location_id(location_ids) unless params.key?(:location_id)

    store_inventory_location_id(params[:location_id].presence&.to_i, location_ids)
  end

  def remembered_inventory_location_id(location_ids)
    remembered_location_id = cookies.signed[:medications_location_id].to_i
    location_ids.include?(remembered_location_id) ? remembered_location_id : nil
  end

  def store_inventory_location_id(selected_location_id, location_ids)
    if selected_location_id.present? && location_ids.include?(selected_location_id)
      cookies.signed[:medications_location_id] = {
        value: selected_location_id,
        httponly: true,
        secure: request.ssl?
      }
      selected_location_id
    else
      cookies.delete(:medications_location_id)
      nil
    end
  end

  def render_medications_index(medication_query:, locations:)
    frame_only = request.headers["Turbo-Frame"] == "medications_inventory"
    render(
      Components::Medications::IndexView.new(
        medications: medication_query.call,
        current_category: @current_category,
        categories: medication_query.categories,
        locations: locations,
        current_location_id: @current_location_id,
        wizard_variant: current_user.wizard_variant,
        frame_only: frame_only
      ),
      layout: !frame_only
    )
  end
end

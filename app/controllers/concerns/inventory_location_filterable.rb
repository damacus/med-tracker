# frozen_string_literal: true

module InventoryLocationFilterable
  extend ActiveSupport::Concern

  private

  def accessible_inventory_locations(medications)
    Location.joins(:medications)
            .merge(medications.except(:includes))
            .distinct
            .order(:name)
            .to_a
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
        httponly: true
      }
      selected_location_id
    else
      cookies.delete(:medications_location_id)
      nil
    end
  end
end

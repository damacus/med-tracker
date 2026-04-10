# frozen_string_literal: true

class MedicationQuery
  attr_reader :scope, :category, :location_id

  def initialize(scope:, category: nil, location_id: nil)
    @scope = scope
    @category = category.presence
    @location_id = location_id.presence
  end

  def call
    filtered_scope.order(:name)
  end

  def categories
    location_filtered_scope.where.not(category: [nil, '']).distinct.order(:category).pluck(:category)
  end

  private

  def filtered_scope
    relation = location_filtered_scope
    relation = relation.where(category: category) if category.present?
    relation
  end

  def location_filtered_scope
    return scope if location_id.blank?

    scope.where(location_id: location_id)
  end
end

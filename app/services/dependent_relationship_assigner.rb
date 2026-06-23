# frozen_string_literal: true

class DependentRelationshipAssigner
  attr_reader :carer, :dependent_ids, :relationship_type, :scope

  def initialize(carer:, dependent_ids:, relationship_type:, scope:)
    @carer = carer
    @dependent_ids = dependent_ids
    @relationship_type = relationship_type
    @scope = scope
  end

  def call
    return [] if carer.blank?

    dependents.map { |dependent| assign(dependent) }
  end

  private

  def dependents
    scope.where(id: selected_ids, person_type: %i[minor dependent_adult], has_capacity: false)
  end

  def selected_ids
    Array(dependent_ids).filter_map do |id|
      id.to_i if id.to_s.match?(/\A\d+\z/)
    end.uniq
  end

  def assign(dependent)
    relationship = CarerRelationship.find_or_initialize_by(carer: carer, patient: dependent)
    relationship.relationship_type = resolved_relationship_type
    relationship.active = true
    relationship.save! if relationship.new_record? || relationship.changed?
    relationship
  end

  def resolved_relationship_type
    relationship_type
  end
end

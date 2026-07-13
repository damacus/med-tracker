# frozen_string_literal: true

class DependentRelationshipAssigner
  attr_reader :carer, :dependent_ids, :relationship_type, :access_level, :scope, :granted_by_membership

  def initialize(carer:, dependent_ids:, relationship_type:, scope:, **options)
    @carer = carer
    @dependent_ids = dependent_ids
    @relationship_type = relationship_type
    @access_level = options[:access_level]
    @scope = scope
    @granted_by_membership = options[:granted_by_membership]
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
    CareDelegation::Assign.new(
      carer: carer,
      patient: dependent,
      relationship_type: resolved_relationship_type,
      access_level: access_level,
      granted_by_membership: granted_by_membership
    ).call
  end

  def resolved_relationship_type
    relationship_type
  end
end

# frozen_string_literal: true

class InventoryLocationsQuery
  attr_reader :medications_scope

  def initialize(medications_scope:)
    @medications_scope = medications_scope
  end

  def call
    Location.joins(:medications)
            .merge(medications_scope.except(:includes))
            .distinct
            .order(:name)
            .to_a
  end
end

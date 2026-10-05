# frozen_string_literal: true

module Admin
  class CarerRelationshipOptionsQuery
    Result = Data.define(:carers, :patients)

    attr_reader :scope

    def initialize(scope:)
      @scope = scope
    end

    def call
      Result.new(
        carers: scope.where(has_capacity: true).order(:name),
        patients: scope.order(:name)
      )
    end
  end
end

# frozen_string_literal: true

module Admin
  class CarerRelationshipsIndexQuery
    attr_reader :scope

    def initialize(scope:)
      @scope = scope
    end

    def call
      scope.includes(:carer, :patient).order(created_at: :desc)
    end
  end
end

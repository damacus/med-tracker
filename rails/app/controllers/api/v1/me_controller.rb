# frozen_string_literal: true

module Api
  module V1
    class MeController < BaseController
      def show
        render json: { data: MeSerializer.new(current_user).as_json }
      end
    end
  end
end

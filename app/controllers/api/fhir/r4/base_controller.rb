# frozen_string_literal: true

module Api
  module Fhir
    module R4
      class BaseController < Api::V1::BaseController
        private

        def render_fhir_collection(scope, serializer)
          records = scope.limit(100).to_a
          render json: ::Fhir::R4::Serializer.bundle(records, type: serializer)
        end

        def render_fhir_resource(record, serializer)
          render json: ::Fhir::R4::Serializer.public_send(serializer, record)
        end
      end
    end
  end
end

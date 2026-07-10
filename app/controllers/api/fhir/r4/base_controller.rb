# frozen_string_literal: true

module Api
  module Fhir
    module R4
      class BaseController < Api::V1::BaseController
        FHIR_JSON = 'application/fhir+json'
        SUPPORTED_FORMATS = ['json', FHIR_JSON].freeze

        before_action :ensure_fhir_format
        before_action :ensure_smart_scope

        private

        def policy_scope(scope, policy_scope_class: nil)
          resolved = super
          return resolved unless current_api_session.is_a?(OauthGrant) && current_api_session.patient_scoped?

          smart_patient_scope(resolved)
        end

        def ensure_smart_scope
          return unless current_api_session.is_a?(OauthGrant)
          return if controller_name == 'metadata'
          return if current_api_session.allows_fhir_read?(controller_name.classify)

          render_forbidden
        end

        def smart_patient_scope(scope)
          person_id = current_api_session.person_id
          case scope.klass.name
          when 'Person'
            scope.where(id: person_id)
          when 'Schedule', 'PersonMedication'
            scope.where(person_id: person_id)
          when 'MedicationTake'
            smart_medication_take_scope(scope, person_id)
          when 'Medication'
            smart_medication_scope(scope, person_id)
          else
            scope.none
          end
        end

        def smart_medication_take_scope(scope, person_id)
          scope.left_outer_joins(:schedule, :person_medication)
               .where(schedules: { person_id: person_id })
               .or(scope.left_outer_joins(:schedule, :person_medication)
                        .where(person_medications: { person_id: person_id }))
        end

        def smart_medication_scope(scope, person_id)
          scope.left_outer_joins(:schedules, :person_medications)
               .where(schedules: { person_id: person_id })
               .or(scope.left_outer_joins(:schedules, :person_medications)
                        .where(person_medications: { person_id: person_id })).distinct
        end

        def render_fhir_collection(scope, serializer, search: {})
          searched = apply_fhir_search(scope, search)
          paginated = paginate_fhir(searched)
          render json: ::Fhir::R4::Serializer.bundle(
            paginated.fetch(:records),
            type: serializer,
            total: paginated.fetch(:total),
            links: bundle_links(paginated)
          ), content_type: FHIR_JSON
        end

        def render_fhir_resource(record, serializer)
          render json: ::Fhir::R4::Serializer.public_send(serializer, record), content_type: FHIR_JSON
        end

        def render_api_error(code:, message:, status:, errors: nil)
          outcome_code = fhir_issue_code(code)
          issue = {
            severity: 'error',
            code: outcome_code,
            details: { text: message },
            diagnostics: request.request_id
          }
          if errors.present?
            issue[:extension] = [{ url: 'https://medtracker.example/fhir/error-details',
                                   valueString: errors.to_json }]
          end

          render json: { resourceType: 'OperationOutcome', issue: [issue] }, status: status, content_type: FHIR_JSON
        end

        def render_not_acceptable(message)
          render_api_error(code: 'not_supported', message: message, status: :not_acceptable)
        end

        def apply_fhir_search(scope, search)
          unsupported = request.query_parameters.keys - supported_query_parameters(search)
          raise InvalidFilterValue, "Unsupported FHIR search parameter: #{unsupported.first}" if unsupported.any?

          search.reduce(scope) do |filtered, (name, callable)|
            value = params[name]
            value.present? ? callable.call(filtered, value) : filtered
          end
        end

        def paginate_fhir(scope)
          page = [params.fetch(:page, 1).to_i, 1].max
          count = params.fetch(:_count, 100).to_i.clamp(1, 100)
          total = scope.count

          {
            page: page,
            count: count,
            total: total,
            records: scope.limit(count).offset((page - 1) * count).to_a
          }
        end

        def bundle_links(paginated)
          links = [{ relation: 'self', url: request.original_url }]
          return links unless paginated.fetch(:page) * paginated.fetch(:count) < paginated.fetch(:total)

          links << { relation: 'next', url: next_page_url(paginated.fetch(:page) + 1) }
        end

        def next_page_url(page)
          query = request.query_parameters.merge('page' => page)
          "#{request.base_url}#{request.path}?#{query.to_query}"
        end

        def supported_query_parameters(search)
          search.keys.map(&:to_s) + %w[_count _format page]
        end

        def ensure_fhir_format
          requested_format = params[:_format].presence
          return if requested_format.blank? || SUPPORTED_FORMATS.include?(requested_format)

          render_not_acceptable("FHIR R4 responses are only available as #{FHIR_JSON}")
        end

        def fhir_issue_code(code)
          case code.to_s
          when 'not_supported'
            'not-supported'
          when 'not_found'
            'not-found'
          when 'forbidden', 'unauthorized'
            'security'
          when 'unprocessable_content'
            'invalid'
          else
            'processing'
          end
        end

        def portable_reference_id(value)
          value.to_s.split('/').last
        end

        def iso8601_date(value, field:)
          Date.iso8601(value.to_s)
        rescue ArgumentError
          raise InvalidFilterValue, "#{field} must be ISO8601"
        end

        def search_by_portable_id
          lambda do |scope, value|
            scope.where(portable_id: portable_reference_id(value))
          end
        end

        def search_by_updated_date
          lambda do |scope, value|
            date = iso8601_date(value, field: 'date')
            scope.where(updated_at: date.all_day)
          end
        end
      end
    end
  end
end

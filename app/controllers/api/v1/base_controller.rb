# frozen_string_literal: true

module Api
  module V1
    class BaseController < ActionController::API
      include Pundit::Authorization

      before_action :authenticate_api_request!

      class InvalidFilterValue < StandardError; end

      rescue_from ActiveRecord::RecordNotFound, with: :render_not_found
      rescue_from InvalidFilterValue, with: :render_invalid_filter
      rescue_from Pundit::NotAuthorizedError, with: :render_forbidden

      attr_reader :current_api_session

      private

      def current_account
        current_api_session&.account
      end

      def current_user
        current_account&.person&.user
      end

      def authenticate_api_request!
        token = bearer_token
        @current_api_session = ApiSession.lookup_by_access_token(token)

        unless @current_api_session&.active_access_token?
          render_unauthorized('Authentication required')
          return
        end

        if current_account.blank? || !current_account.verified? || current_user.blank? || !current_user.active?
          render_unauthorized('Authentication required')
          return
        end

        @current_api_session.touch_last_used!
      end

      def bearer_token
        auth_header = request.headers['Authorization'].to_s
        scheme, token = auth_header.split(' ', 2)
        return nil unless scheme == 'Bearer'

        token
      end

      def render_collection(scope, serializer:, includes: nil)
        records = apply_collection_filters(scope)
        paginated = paginate(records, includes:)

        render json: {
          data: paginated[:records].map { |record| serializer.new(record).as_json },
          meta: paginated[:meta]
        }
      end

      def render_resource(record, serializer:)
        render json: { data: serializer.new(record).as_json }
      end

      def apply_collection_filters(scope)
        filtered = scope.order(:id)
        return filtered unless params[:updated_since].present? && scope.klass.column_names.include?('updated_at')

        filtered.where(scope.klass.arel_table[:updated_at].gteq(Time.iso8601(params[:updated_since])))
      rescue ArgumentError
        raise InvalidFilterValue, 'updated_since must be ISO8601'
      end

      def paginate(scope, includes: nil)
        page = [params.fetch(:page, 1).to_i, 1].max
        per_page = params.fetch(:per_page, 20).to_i.clamp(1, 100)

        relation = includes ? scope.includes(*Array(includes)) : scope
        total_count = relation.count
        records = relation.limit(per_page).offset((page - 1) * per_page)

        {
          records: records,
          meta: {
            page: page,
            per_page: per_page,
            total_count: total_count
          }
        }
      end

      def render_not_found
        render json: { error: { code: 'not_found', message: 'Record not found' } }, status: :not_found
      end

      def render_forbidden
        render json: { error: { code: 'forbidden', message: 'You are not authorized to perform this action.' } },
               status: :forbidden
      end

      def render_unauthorized(message)
        render json: { error: { code: 'unauthorized', message: message } }, status: :unauthorized
      end

      def render_unprocessable(message)
        render json: { error: { code: 'unprocessable_content', message: message } }, status: :unprocessable_content
      end

      def render_invalid_filter(exception)
        render_unprocessable(exception.message)
      end
    end
  end
end

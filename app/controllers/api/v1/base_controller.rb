# frozen_string_literal: true

module Api
  module V1
    class BaseController < ActionController::API
      include Pundit::Authorization

      around_action :with_api_request_context

      class InvalidFilterValue < StandardError; end

      rescue_from ActiveRecord::RecordNotFound, with: :render_not_found
      rescue_from InvalidFilterValue, with: :render_invalid_filter
      rescue_from Pundit::NotAuthorizedError, with: :render_forbidden

      attr_reader :current_api_session, :current_household, :current_membership

      private

      def with_api_request_context
        authenticate_api_request!
        return if performed?

        TenantContext.with(account: current_account, household: nil, request_id: request.request_id) do
          bind_api_session_context!
          bind_api_household_context! unless performed?
          yield unless performed?
        end
      ensure
        Current.reset
      end

      def current_account
        current_api_session&.account
      end

      def current_user
        current_account&.person&.user
      end

      def pundit_user
        AuthorizationContext.current || current_user
      end

      def authenticate_api_request!
        @current_api_session = lookup_api_credential
        return render_unauthorized('Authentication required') unless valid_session?
        return render_unauthorized('Authentication required') unless valid_account_and_user?
        return render_unauthorized('Authentication required') if ApiAuthState.locked_out?(current_account)

        Current.account = current_account
        Current.request_id = request.request_id
        @current_api_session.touch_last_used!
      end

      def valid_session?
        return false if @current_api_session.blank? || @current_api_session.revoked_at.present?
        return @current_api_session.access_expires_at.future? if @current_api_session.is_a?(ApiSession)

        @current_api_session.is_a?(ApiAppToken)
      end

      def valid_account_and_user?
        current_account.present? && current_account.verified? && current_user.present? && current_user.active?
      end

      def bind_api_household_context!
        return if params[:household_id].blank?

        @current_household = Household.find(params.expect(:household_id))
        @current_membership = @current_api_session.household_membership
        return render_forbidden unless @current_membership&.active?
        return render_forbidden unless @current_membership.household_id == @current_household.id

        Current.household = @current_household
        Current.membership = @current_membership
        TenantContext.set_household!(@current_household)
        TenantContext.set_membership!(@current_membership)
      end

      def bind_api_session_context!
        @current_membership = @current_api_session.household_membership
        return render_unauthorized('Authentication required') unless @current_api_session.active_for_membership?

        @current_household = @current_membership&.household
        Current.household = @current_household
        Current.membership = @current_membership
        TenantContext.set_household!(@current_household)
        TenantContext.set_membership!(@current_membership)
      end

      def bearer_token
        auth_header = request.headers['Authorization'].to_s
        scheme, token = auth_header.split(' ', 2)
        return nil unless scheme == 'Bearer'

        token
      end

      def lookup_api_credential
        token = bearer_token
        ApiSession.lookup_by_access_token(token) || ApiAppToken.lookup_by_token(token)
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

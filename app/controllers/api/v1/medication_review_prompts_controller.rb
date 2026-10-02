module Api
  module V1
    class MedicationReviewPromptsController < BaseController
      rescue_from MedicationReviews::UpdatePrompt::VersionError, with: :render_version_error
      rescue_from ActiveRecord::RecordInvalid, with: :render_invalid_review

      def index
        authorize MedicationReviewPrompt
        scope = filtered_scope
        MedicationReviewPromptSync.new(people: policy_scope(Person)).call
        page = paginate(scope)
        render json: { data: page[:records].map { |record| MedicationReviewPromptSerializer.new(record).as_json },
                       meta: page[:meta] }
      rescue ArgumentError
        render_unprocessable('Review filters are invalid')
      end

      def show
        authorize prompt
        render_resource(prompt, serializer: MedicationReviewPromptSerializer)
      end

      def update
        attributes = params.expect(medication_review_prompt: MedicationReviews::UpdatePrompt::ATTRIBUTES)
        record = MedicationReviews::UpdatePrompt.new(prompt: prompt, authorization: pundit_user)
                                                .call(attributes: attributes, if_match: request.headers['If-Match'])
        render_resource(record, serializer: MedicationReviewPromptSerializer)
      end

      private

      def with_api_idempotency(&)
        response.set_header('Cache-Control', 'no-store')
        authorize prompt, :update? if action_name == 'update'
        super
      end

      def prompt
        @prompt ||= policy_scope(MedicationReviewPrompt).find(params.expect(:id))
      end

      def filtered_scope
        MedicationReviewPromptQuery.new(scope: policy_scope(MedicationReviewPrompt),
                                        review_status: params[:review_status].presence || 'needs_review',
                                        priority: params[:priority].presence || 'all',
                                        show_hidden: params[:show_hidden] == '1').call
      end

      def render_version_error(error)
        status = error.code == 'precondition_required' ? :precondition_required : :conflict
        render_api_error(code: error.code, message: error.message, status: status)
      end

      def render_invalid_review
        render_unprocessable('Review could not be saved')
      end
    end
  end
end

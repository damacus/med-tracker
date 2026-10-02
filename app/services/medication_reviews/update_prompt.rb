module MedicationReviews
  class UpdatePrompt
    ATTRIBUTES = %i[status practitioner_name practitioner_role reviewed_on review_note].freeze

    class VersionError < StandardError
      attr_reader :code

      def initialize(code)
        @code = code
        super('A current review version is required')
      end
    end

    def initialize(prompt:, authorization:)
      @prompt = prompt
      @authorization = authorization
    end

    def call(attributes:, if_match:)
      Households::LifecycleCutoffLock.with(household_id: @prompt.household_id) do
        @prompt.with_lock(requires_new: true) do
          authorize_current_actor!
          validate_version!(if_match)
          update_prompt(attributes)
        end
      end
      @prompt
    end

    private

    def authorize_current_actor!
      raise Pundit::NotAuthorizedError unless @prompt.household.reload.operational?

      current_context = @authorization.with(membership: @authorization.membership.reload)
      Pundit.authorize(current_context, @prompt, :update?)
    end

    def validate_version!(expected)
      raise VersionError, 'precondition_required' if expected.blank?
      raise VersionError, 'conflict' unless expected == Api::RecordEtag.for(@prompt)
    end

    def update_prompt(attributes)
      previous_status = @prompt.status
      @prompt.assign_attributes(attributes.to_h.symbolize_keys.slice(*ATTRIBUTES))
      @prompt.reviewed_by_membership = @authorization.membership if @prompt.practitioner_review_status?
      @prompt.save!
      Audit::Event.record!(household: @prompt.household, event_type: 'medication_review_prompt.updated',
                           metadata: { prompt_id: @prompt.id, person_id: @prompt.person_id,
                                       previous_status: previous_status, status: @prompt.status })
    end
  end
end

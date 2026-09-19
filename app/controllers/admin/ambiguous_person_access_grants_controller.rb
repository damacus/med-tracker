# frozen_string_literal: true

module Admin
  class AmbiguousPersonAccessGrantsController < BaseController
    def index
      authorize PersonAccessGrant, :index?
      grants = Admin::AmbiguousPersonAccessGrantsIndexQuery.new(scope: policy_scope(PersonAccessGrant)).call
      @pagy, grants = pagy(:offset, grants)

      render Components::Admin::AmbiguousPersonAccessGrants::IndexView.new(
        grants: grants,
        pagy: @pagy
      )
    end

    def classify
      grant = current_household.person_access_grants.find(params.expect(:id))
      authorize grant, :classify?
      return redirect_to_queue(alert: t('.mfa_required')) unless mfa_satisfied?

      Admin::AmbiguousPersonAccessGrantClassifier.new(
        grant: grant,
        actor_membership: current_membership,
        request: request
      ).call(
        decision: classify_params[:decision],
        reason: classify_params[:reason],
        carer_relationship_id: classify_params[:carer_relationship_id]
      )

      redirect_to_queue(notice: t(".#{classify_params[:decision]}_notice"))
    rescue Admin::AmbiguousPersonAccessGrantClassifier::Error, ActiveRecord::RecordInvalid => e
      redirect_to_queue(alert: e.message)
    end

    private

    def classify_params
      params.expect(classification: %i[decision reason carer_relationship_id])
    end

    def mfa_satisfied?
      ApiAuthState.web_session_mfa_satisfied?(session, current_account)
    end

    def redirect_to_queue(**flash_args)
      redirect_to admin_ambiguous_person_access_grants_path, **flash_args
    end
  end
end

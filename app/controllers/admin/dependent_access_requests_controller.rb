# frozen_string_literal: true

module Admin
  # Reviews parent requests for access to dependents.
  class DependentAccessRequestsController < ApplicationController
    def index
      authorize DependentAccessRequest
      requests = policy_scope(DependentAccessRequest).pending.recent_first.includes(:requester, :carer, :patient)

      render plain: requests.map { |request| request_summary(request) }.join("\n")
    end

    def approve
      request_record = policy_scope(DependentAccessRequest).find(params.expect(:id))
      authorize request_record
      request_record.approve!(reviewer: current_user)

      redirect_to admin_dependent_access_requests_path,
                  notice: t('admin.dependent_access_requests.approved', default: 'Dependent access request approved.')
    end

    def reject
      request_record = policy_scope(DependentAccessRequest).find(params.expect(:id))
      authorize request_record
      request_record.reject!(reviewer: current_user)

      redirect_to admin_dependent_access_requests_path,
                  notice: t('admin.dependent_access_requests.rejected', default: 'Dependent access request rejected.')
    end

    private

    def request_summary(request)
      "##{request.id}: #{request.carer.name} requested access to #{request.patient.name}"
    end
  end
end

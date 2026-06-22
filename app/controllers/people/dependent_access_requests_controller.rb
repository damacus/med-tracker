# frozen_string_literal: true

module People
  # Allows parents to request administrator approval for dependent access.
  class DependentAccessRequestsController < ApplicationController
    def create
      patient = Person.find(params.expect(:person_id))
      request_record = DependentAccessRequest.new(
        requester: current_user,
        carer: current_user.person,
        patient: patient,
        relationship_type: 'parent'
      )
      authorize request_record

      if request_record.save
        redirect_to dashboard_path,
                    notice: t('dependent_access_requests.created',
                              default: 'Access request sent for administrator approval.')
      else
        redirect_to dashboard_path, alert: request_record.errors.full_messages.to_sentence
      end
    end
  end
end

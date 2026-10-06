# frozen_string_literal: true

module People
  class AvatarsController < ApplicationController
    before_action :require_authentication
    before_action :check_two_factor_setup

    def show
      person = policy_scope(Person).find(params.expect(:person_id))
      authorize person, :show?

      return head :not_found unless person.avatar.attached?

      send_data person.avatar.download,
                type: person.avatar.content_type,
                disposition: 'inline',
                filename: person.avatar.filename.to_s
    end
  end
end

module Profiles
  class Update
    ATTRIBUTES = %w[date_of_birth time_zone gravatar_enabled mobile_shortcuts].freeze
    class InvalidAttributes < StandardError
    end

    def initialize(person:, account:, authorization:)
      @person = person
      @account = account
      @authorization = authorization
    end

    def call(attributes, supplied_keys:)
      validate_keys!(attributes, supplied_keys)
      validate_values!(attributes)
      Households::LifecycleCutoffLock.with(household_id: @person.household_id) do
        save_profile!(attributes)
      end
    end

    private

    def validate_keys!(attributes, supplied_keys)
      raise InvalidAttributes if (supplied_keys.map(&:to_s) - ATTRIBUTES).any?
      raise InvalidAttributes unless attributes.keys.map(&:to_s).sort == supplied_keys.map(&:to_s).sort
    end

    def save_profile!(attributes)
      @account.with_lock(requires_new: true) do
        @person.with_lock do
          authorize_current_actor!
          @person.update!(attributes.slice('date_of_birth'))
          @account.update!(attributes.except('date_of_birth'))
        end
      end
    end

    def authorize_current_actor!
      membership = @authorization.membership.reload
      raise Pundit::NotAuthorizedError unless @person.household.reload.operational?
      raise Pundit::NotAuthorizedError unless membership.person_id == @person.id && @person.account_id == @account.id

      Pundit.authorize(@authorization.with(membership: membership), @person, :update?)
    end

    def validate_values!(attributes)
      validate_date!(attributes['date_of_birth']) if attributes.key?('date_of_birth')
      if attributes.key?('gravatar_enabled') && [true, false].exclude?(attributes['gravatar_enabled'])
        raise InvalidAttributes
      end
      return unless attributes.key?('time_zone') && !attributes['time_zone'].is_a?(String)

      raise InvalidAttributes
    end

    def validate_date!(date)
      raise InvalidAttributes unless date.is_a?(String) && date.match?(/\A\d{4}-\d{2}-\d{2}\z/)

      Date.iso8601(date)
    rescue ArgumentError
      raise InvalidAttributes
    end
  end
end

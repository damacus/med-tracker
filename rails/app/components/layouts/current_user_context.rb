# frozen_string_literal: true

module Components
  module Layouts
    module CurrentUserContext
      attr_reader :current_user, :membership, :household

      def initialize(current_user: nil, membership: nil, household: nil)
        @current_user = current_user
        @membership = membership
        @household = household
        super()
      end

      private

      def authenticated?
        current_user.present?
      end

      def user_is_admin?
        current_membership&.owner? || current_membership&.administrator? || false
      end

      def current_user_name
        current_user&.name
      end

      def current_membership_role_name
        (current_membership&.role.presence || 'member').to_s.humanize
      end

      def current_membership
        membership || Current.membership
      end

      def current_household
        household || Current.household
      end
    end
  end
end

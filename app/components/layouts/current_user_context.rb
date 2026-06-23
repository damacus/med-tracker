# frozen_string_literal: true

module Components
  module Layouts
    module CurrentUserContext
      attr_reader :current_user

      def initialize(current_user: nil)
        @current_user = current_user
        super()
      end

      private

      def authenticated?
        current_user.present?
      end

      def user_is_admin?
        Current.membership&.owner? || Current.membership&.administrator? || false
      end

      def current_user_name
        current_user&.name
      end

      def current_membership_role_name
        (Current.membership&.role.presence || 'member').to_s.humanize
      end
    end
  end
end

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
        current_user&.administrator? || false
      end

      def current_user_name
        current_user&.name
      end

      def current_user_initials
        return 'U' if current_user_name.blank?

        current_user_name.split.map(&:first).join.upcase
      end
    end
  end
end

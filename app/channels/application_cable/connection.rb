# frozen_string_literal: true

module ApplicationCable
  class Connection < ActionCable::Connection::Base
    identified_by :current_user

    def connect
      set_current_user || reject_unauthorized_connection
    end

    private

    def set_current_user
      # Rodauth stores account_id in the session
      account_id = request.session[:account_id]
      return unless account_id

      account = Account.find_by(id: account_id)
      return unless account

      self.current_user = account.person&.user
    end
  end
end

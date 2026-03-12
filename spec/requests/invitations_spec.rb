# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Invitations' do
  describe 'GET /invitations/accept' do
    it 'rejects existing minor invitation tokens' do
      invitation = Invitation.new(email: 'child.invite@example.com', role: :minor)
      invitation.save!(validate: false)

      get accept_invitation_path(token: invitation.token)

      expect(response).to have_http_status(:not_found)
      expect(response.body).to include('This invitation link is invalid or has expired.')
    end

    it 'rejects an old token after an invitation is resent' do
      invitation = create(:invitation)
      original_token = invitation.token

      invitation.resend!

      get accept_invitation_path(token: original_token)

      expect(response).to have_http_status(:not_found)
      expect(response.body).to include('This invitation link is invalid or has expired.')
    end
  end
end

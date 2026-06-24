# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Active Storage direct uploads' do
  it 'does not expose the default unauthenticated direct upload endpoint' do
    expect do
      post '/rails/active_storage/direct_uploads', params: {
        blob: {
          filename: 'avatar.png',
          byte_size: 6,
          checksum: Base64.strict_encode64(Digest::MD5.digest('avatar')),
          content_type: 'image/png'
        }
      }, as: :json
    end.not_to change(ActiveStorage::Blob, :count)

    expect(response).to have_http_status(:not_found).or have_http_status(:unauthorized).or have_http_status(:forbidden)
  end
end

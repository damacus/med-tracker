# frozen_string_literal: true

class ApiIdempotencyKey < ApplicationRecord
  belongs_to :household
  belongs_to :account
  belongs_to :api_session, optional: true
  belongs_to :api_app_token, optional: true

  validates :key, :request_method, :request_path, :request_digest, :response_status, :expires_at, presence: true
  validates :key, uniqueness: { scope: :household_id }
end

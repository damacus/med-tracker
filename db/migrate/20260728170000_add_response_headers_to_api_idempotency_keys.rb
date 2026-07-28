# frozen_string_literal: true

class AddResponseHeadersToApiIdempotencyKeys < ActiveRecord::Migration[8.1]
  def change
    add_column :api_idempotency_keys, :response_headers, :jsonb, default: {}, null: false
  end
end

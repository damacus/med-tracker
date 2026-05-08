# frozen_string_literal: true

class ExternalLookupAuditEvent < ApplicationRecord
  SOURCES = %w[nhs_dmd open_food_facts nhs_website_content].freeze
  EVENTS = %w[search barcode_lookup medicine_guidance_lookup].freeze
  RESULT_STATUSES = %w[success not_found not_configured error].freeze

  validates :source, presence: true, inclusion: { in: SOURCES }
  validates :event, presence: true, inclusion: { in: EVENTS }
  validates :result_status, presence: true, inclusion: { in: RESULT_STATUSES }
  validates :result_count, numericality: { only_integer: true, greater_than_or_equal_to: 0 }
end

# frozen_string_literal: true

class NotificationEvent < ApplicationRecord
  belongs_to :household
  belongs_to :person, optional: true

  validates :event_type, presence: true
  validates :event_key, presence: true, uniqueness: { scope: :event_type }

  def self.record_once!(household:, person:, event_type:, event_key:, metadata: {})
    create!(
      household: household,
      person: person,
      event_type: event_type,
      event_key: event_key,
      metadata: metadata
    )
  rescue ActiveRecord::RecordNotUnique
    nil
  rescue ActiveRecord::RecordInvalid => e
    raise unless e.record.errors.of_kind?(:event_key, :taken)

    nil
  end
end

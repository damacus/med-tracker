# frozen_string_literal: true

module ActiveStorageAttachmentTenancy
  extend ActiveSupport::Concern

  included do
    belongs_to :household, optional: true

    before_validation :assign_household_from_record
    validate :household_matches_record
  end

  private

  def assign_household_from_record
    return if household_id.present?
    return unless record.respond_to?(:household_id)

    self.household_id = record.household_id
  end

  def household_matches_record
    return unless record.respond_to?(:household_id)
    return if household_id == record.household_id

    errors.add(:household, 'must match attached record household')
  end
end

Rails.application.config.to_prepare do
  ActiveStorage::Attachment.include(ActiveStorageAttachmentTenancy) unless ActiveStorage::Attachment < ActiveStorageAttachmentTenancy
end

# frozen_string_literal: true

module PortableIdentifiable
  extend ActiveSupport::Concern

  included do
    include SyncTrackable

    before_validation :assign_portable_id, on: :create

    validates :portable_id, presence: true, uniqueness: { scope: :household_id }
    validate :portable_id_is_immutable
  end

  private

  def assign_portable_id
    self.portable_id ||= SecureRandom.uuid
  end

  def portable_id_is_immutable
    return unless persisted? && will_save_change_to_portable_id?

    errors.add(:portable_id, 'cannot be changed')
  end
end

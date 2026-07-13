# frozen_string_literal: true

module SyncTrackable
  extend ActiveSupport::Concern

  included do
    after_create -> { record_sync_change('create') }
    after_update -> { record_sync_change('update') }
    after_touch -> { record_sync_change('update') }
    after_destroy -> { record_sync_change('delete') }
  end

  def refresh_sync_version!
    PaperTrail.request(enabled: false) do
      self.updated_at = Time.current
      save!(validate: false)
    end
  end

  def record_sync_deletion!
    record_sync_change('delete')
  end

  private

  def record_sync_change(action)
    return unless Current.household&.id == household_id

    Api::ChangeRecorder.new(
      household: household,
      account: Current.account,
      membership: Current.membership,
      request_id: Current.request_id
    ).record(self, action: action)
  end
end

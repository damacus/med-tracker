# frozen_string_literal: true

class NhsDmdImport < ApplicationRecord
  ACTIVE_STATUSES = %i[queued extracting counting importing].freeze

  PROGRESS_COUNTER_KEYS = %i[
    total_records
    processed_records
    imported_count
    skipped_count
    created_count
    updated_count
    unchanged_count
    skipped_expired_count
    skipped_missing_name_count
    skipped_invalid_count
  ].freeze
  ARCHIVE_REFERENCE_FIELDS = %i[
    archive_service_name archive_key archive_checksum archive_byte_size
  ].freeze

  enum :status, {
    queued: 0,
    extracting: 1,
    counting: 2,
    importing: 3,
    completed: 4,
    failed: 5
  }, default: :queued, validate: true

  validates :uploaded_filename, presence: true
  validate :complete_archive_reference

  after_update_commit :broadcast_refresh

  def self.latest_first
    order(created_at: :desc)
  end

  def persist_archive!(uploaded_file, store: NhsDmd::ArchiveStore.new, service_name: nil)
    options = { import_run: self, uploaded_file: }
    options[:service_name] = service_name if service_name
    store.persist(**options)
  end

  def archive_reference?
    archive_reference_values.all?(&:present?)
  end

  def legacy_archive?
    archive_path.present? && !archive_reference?
  end

  def start!
    update!(started_at: Time.current) if started_at.blank?
  end

  def apply_progress!(progress)
    update!(progress_attributes(progress))
  end

  def complete!(result)
    update!(
      status: :completed,
      processed_records: final_processed_records(result),
      imported_count: result.imported_count,
      skipped_count: result.skipped_count,
      created_count: result.created_count,
      updated_count: result.updated_count,
      unchanged_count: result.unchanged_count,
      skipped_expired_count: result.skipped_expired_count,
      skipped_missing_name_count: result.skipped_missing_name_count,
      skipped_invalid_count: result.skipped_invalid_count,
      completed_at: Time.current,
      error_message: nil
    )
  end

  def fail!(message)
    update!(
      status: :failed,
      completed_at: Time.current,
      error_message: message,
      log: appended_log(message)
    )
  end

  def progress_percentage
    return 0 if total_records.to_i <= 0

    ((processed_records.to_f / total_records) * 100).floor
  end

  def active?
    queued? || extracting? || counting? || importing?
  end

  private

  def final_processed_records(result)
    return total_records if total_records.positive?

    result.imported_count + result.skipped_count + result.unchanged_count
  end

  def complete_archive_reference
    return if archive_reference_values.none?(&:present?) || archive_reference?

    ARCHIVE_REFERENCE_FIELDS.select { public_send(it).blank? }.each { errors.add(it, :blank) }
  end

  def archive_reference_values
    ARCHIVE_REFERENCE_FIELDS.map { public_send(it) }
  end

  def appended_log(message)
    [log.presence, message].compact.join("\n")
  end

  def broadcast_refresh
    broadcast_refresh_to(self)
  end

  def progress_attributes(progress)
    normalized = progress.symbolize_keys

    counter_attrs = PROGRESS_COUNTER_KEYS.index_with { |key| value_or_current(normalized, key) }

    {
      status: normalized[:status].presence,
      started_at: started_at || Time.current,
      log: normalized[:message].present? ? appended_log(normalized[:message]) : log
    }.merge(counter_attrs).compact
  end

  def value_or_current(progress, key)
    return public_send(key) if progress[key].nil?

    progress[key]
  end
end

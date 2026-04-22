# frozen_string_literal: true

require 'fileutils'

class NhsDmdImport < ApplicationRecord
  enum :status, {
    queued: 0,
    extracting: 1,
    counting: 2,
    importing: 3,
    completed: 4,
    failed: 5
  }, default: :queued, validate: true

  validates :uploaded_filename, presence: true

  def self.latest_first
    order(created_at: :desc)
  end

  def persist_archive!(uploaded_file)
    source_path = uploaded_file.respond_to?(:path) ? uploaded_file.path : uploaded_file.to_s
    raise ArgumentError, 'Import archive is missing.' if source_path.blank?

    FileUtils.mkdir_p(archive_directory)
    FileUtils.cp(source_path, archive_destination_path)
    update!(archive_path: archive_destination_path.to_s)
  end

  def start!
    update!(started_at: Time.current) if started_at.blank?
  end

  def apply_progress!(progress)
    update!(progress_attributes(progress))
  end

  def complete!(result)
    final_total = total_records.positive? ? total_records : result.imported_count + result.skipped_count

    update!(
      status: :completed,
      processed_records: final_total,
      imported_count: result.imported_count,
      skipped_count: result.skipped_count,
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

  def archive_directory
    Rails.root.join('storage', 'nhs_dmd', 'imports', id.to_s)
  end

  def archive_destination_path
    archive_directory.join(sanitized_filename)
  end

  def sanitized_filename
    File.basename(uploaded_filename.to_s).gsub(/[^A-Za-z0-9.\-_]/, '_').presence || 'release.zip'
  end

  def appended_log(message)
    [log.presence, message].compact.join("\n")
  end

  def progress_attributes(progress)
    normalized = progress.symbolize_keys

    {
      status: normalized[:status].presence,
      started_at: started_at || Time.current,
      total_records: value_or_current(normalized, :total_records),
      processed_records: value_or_current(normalized, :processed_records),
      imported_count: value_or_current(normalized, :imported_count),
      skipped_count: value_or_current(normalized, :skipped_count),
      log: normalized[:message].present? ? appended_log(normalized[:message]) : log
    }.compact
  end

  def value_or_current(progress, key)
    return public_send(key) if progress[key].nil?

    progress[key]
  end
end

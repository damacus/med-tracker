# frozen_string_literal: true

class NhsDmdImportJob < ApplicationJob
  queue_as :default

  def perform(import_run_or_id)
    import_run = resolve_import_run(import_run_or_id)
    perform_import(import_run)
  rescue NhsDmd::ReleaseArchiveImport::Error => e
    fail_import(import_run, e)
  rescue StandardError => e
    fail_import(import_run, e)
    Rails.logger.error("NhsDmdImportJob failed: #{e.class}: #{e.message}")
  end

  private

  def perform_import(import_run)
    import_run.start!
    result = archive_importer.import(import_run.archive_path, progress_callback: progress_callback_for(import_run))
    import_run.reload.complete!(result)
  end

  def resolve_import_run(import_run_or_id)
    return import_run_or_id if import_run_or_id.is_a?(NhsDmdImport)

    NhsDmdImport.find(import_run_or_id)
  end

  def archive_importer
    @archive_importer ||= NhsDmd::ReleaseArchiveImport.new
  end

  def progress_callback_for(import_run)
    lambda do |progress|
      import_run.reload.apply_progress!(progress)
    end
  end

  def fail_import(import_run, error)
    import_run&.reload&.fail!(error.message)
  end
end

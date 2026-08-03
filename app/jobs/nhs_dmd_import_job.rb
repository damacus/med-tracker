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
    Observability::DiagnosticEvent.emit(
      component: :nhs_dmd_import,
      reason: :operation_failed,
      severity: :error,
      error: e
    )
  end

  private

  def perform_import(import_run)
    import_run.start!
    result = archive_store.open(import_run) do |archive_path|
      archive_importer.import(archive_path, progress_callback: progress_callback_for(import_run))
    end
    import_run.reload.complete!(result)
    cleanup_archive(import_run)
  end

  def resolve_import_run(import_run_or_id)
    return import_run_or_id if import_run_or_id.is_a?(NhsDmdImport)

    NhsDmdImport.find(import_run_or_id)
  end

  def archive_importer
    @archive_importer ||= NhsDmd::ReleaseArchiveImport.new
  end

  def archive_store
    @archive_store ||= NhsDmd::ArchiveStore.new
  end

  def progress_callback_for(import_run)
    lambda do |progress|
      import_run.reload.apply_progress!(progress)
    end
  end

  def fail_import(import_run, error)
    import_run&.reload&.fail!(error.message)
    cleanup_archive(import_run) if import_run
  end

  def cleanup_archive(import_run)
    archive_store.cleanup(import_run)
  rescue NhsDmd::ArchiveStore::Error => e
    Observability::DiagnosticEvent.emit(
      component: :nhs_dmd_import,
      reason: :operation_failed,
      severity: :error,
      error: e
    )
  end
end

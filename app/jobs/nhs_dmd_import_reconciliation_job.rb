class NhsDmdImportReconciliationJob < ApplicationJob
  INTERRUPTION_MESSAGE = 'Import interrupted because the worker stopped reporting progress.'

  def perform
    NhsDmdImport.where(status: NhsDmdImport.statuses.values_at(*NhsDmdImport::ACTIVE_STATUSES))
      .where(updated_at: ..30.minutes.ago)
      .find_each { |import_run| import_run.fail!(INTERRUPTION_MESSAGE) }
  end
end

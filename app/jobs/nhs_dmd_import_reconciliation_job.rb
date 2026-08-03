# frozen_string_literal: true

class NhsDmdImportReconciliationJob < ApplicationJob
  INTERRUPTION_MESSAGE = 'Import interrupted because the worker stopped reporting progress.'

  def perform
    active_imports = NhsDmdImport.where(status: NhsDmdImport.statuses.values_at(*NhsDmdImport::ACTIVE_STATUSES))

    active_imports.where(updated_at: ..30.minutes.ago).find_each do |import_run|
      import_run.fail!(INTERRUPTION_MESSAGE)
    end
  end
end

# frozen_string_literal: true

module Admin
  class NhsDmdImportsController < ApplicationController
    def new
      authorize :admin_nhs_dmd_import, :new?

      render Components::Admin::NhsDmdImports::FormView.new(import_run: NhsDmdImport.latest_first.first)
    end

    def create
      authorize :admin_nhs_dmd_import, :create?

      uploaded_file = nhs_dmd_import_params[:release_zip]
      if uploaded_file.blank?
        redirect_to new_admin_nhs_dmd_import_path, alert: t('admin.nhs_dmd_imports.missing_file')
        return
      end

      import_run = NhsDmdImport.create!(
        uploaded_filename: uploaded_file.original_filename.presence || File.basename(uploaded_file.path)
      )
      import_run.persist_archive!(uploaded_file)
      NhsDmdImportJob.perform_later(import_run)

      redirect_to new_admin_nhs_dmd_import_path,
                  notice: t('admin.nhs_dmd_imports.started')
    rescue ArgumentError, ActiveRecord::ActiveRecordError, SystemCallError => e
      import_run&.fail!(e.message) if import_run&.queued?
      redirect_to new_admin_nhs_dmd_import_path, alert: e.message
    end

    private

    def nhs_dmd_import_params
      params.expect(nhs_dmd_import: [:release_zip])
    end
  end
end

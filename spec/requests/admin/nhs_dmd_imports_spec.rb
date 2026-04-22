# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Admin::NhsDmdImports' do
  fixtures :all

  let(:admin) { users(:admin) }
  let(:regular_user) { users(:jane) }

  describe 'GET /admin' do
    before { sign_in(admin) }

    it 'includes a quick action link to import an NHS dm+d release' do
      get admin_root_path

      expect(response).to have_http_status(:success)
      expect(response.body).to include('Import dm+d release')
      expect(response.body).to include(new_admin_nhs_dmd_import_path)
    end
  end

  describe 'GET /admin/nhs_dmd_import/new' do
    context 'when authenticated as administrator' do
      before { sign_in(admin) }

      it 'renders the import form' do
        get new_admin_nhs_dmd_import_path

        expect(response).to have_http_status(:success)
        expect(response.body).to include('Import NHS dm+d release')
        expect(response.body).to include('type="file"')
      end

      it 'renders the latest import run log and status' do
        import = NhsDmdImport.create!(
          uploaded_filename: 'nhsbsa_dmd_release.zip',
          status: :importing,
          log: "Queued import\nProcessed 500 records",
          total_records: 900,
          processed_records: 500,
          imported_count: 480,
          skipped_count: 20
        )

        get new_admin_nhs_dmd_import_path

        expect(response.body).to include('Queued import')
        expect(response.body).to include('Processed 500 records')
        expect(response.body).to include('Importing')
        expect(response.body).to include(import.uploaded_filename)
        expect(response.body).to include('500 / 900')
      end

      it 'renders the latest import log in a scrollable panel' do
        log_lines = (1..12).map { |index| format('Entry %<index>02d', index: index) }.join("\n")

        NhsDmdImport.create!(
          uploaded_filename: 'nhsbsa_dmd_release.zip',
          status: :importing,
          log: log_lines,
          total_records: 12,
          processed_records: 12,
          imported_count: 12,
          skipped_count: 0
        )

        get new_admin_nhs_dmd_import_path

        expect(response.body).to include('Entry 01')
        expect(response.body).to include('Entry 02')
        expect(response.body).to include('Entry 03')
        expect(response.body).to include('Entry 12')
        expect(response.body).to include('max-h-60 overflow-y-auto')
      end
    end

    context 'when authenticated as non-administrator' do
      before { sign_in(regular_user) }

      it 'denies access' do
        get new_admin_nhs_dmd_import_path

        expect(response).to redirect_to(root_path)
      end
    end
  end

  describe 'POST /admin/nhs_dmd_import' do
    before { sign_in(admin) }

    it 'creates an import run, enqueues the job, and redirects with a start notice' do
      upload = uploaded_zip('nhsbsa_dmd_release.zip')
      import_run = nil

      allow(NhsDmdImportJob).to receive(:perform_later) do |record|
        import_run = record
      end

      post admin_nhs_dmd_import_path, params: { nhs_dmd_import: { release_zip: upload } }

      expect(response).to redirect_to(new_admin_nhs_dmd_import_path)
      expect(flash[:notice]).to eq('NHS dm+d import started. Progress and logs will appear below.')
      expect(import_run).to be_a(NhsDmdImport)
      expect(import_run.uploaded_filename).to eq('nhsbsa_dmd_release.zip')
      expect(import_run).to be_queued
      expect(import_run.archive_path).to be_present
      expect(File.exist?(import_run.archive_path)).to be(true)
    end

    it 'redirects with an alert when no file is provided' do
      post admin_nhs_dmd_import_path, params: { nhs_dmd_import: { release_zip: nil } }

      expect(response).to redirect_to(new_admin_nhs_dmd_import_path)
      expect(flash[:alert]).to eq('Select an NHS dm+d release ZIP to import.')
    end

    it 'marks the import as failed when the archive cannot be persisted' do
      upload = uploaded_zip('broken_release.zip')
      failed_import = nil

      allow(NhsDmdImport).to receive(:create!).and_wrap_original do |original, *args|
        original.call(*args).tap do |import_run|
          failed_import = import_run
          allow(import_run).to receive(:persist_archive!).and_raise(SystemCallError.new('disk full'))
        end
      end

      expect do
        post admin_nhs_dmd_import_path, params: { nhs_dmd_import: { release_zip: upload } }
      end.to change(NhsDmdImport, :count).by(1)

      expect(response).to redirect_to(new_admin_nhs_dmd_import_path)
      expect(flash[:alert]).to include('disk full')
      expect(failed_import.reload).to be_failed
      expect(failed_import).not_to be_active
    end
  end

  def uploaded_zip(filename)
    file = Tempfile.create([File.basename(filename, '.zip'), '.zip'])
    file.tap do
      file.write('zip')
      file.rewind
    end

    Rack::Test::UploadedFile.new(file.path, 'application/zip', original_filename: filename)
  end
end

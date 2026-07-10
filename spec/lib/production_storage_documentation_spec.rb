# frozen_string_literal: true

require 'rails_helper'

RSpec.describe ProductionStorage, '.documentation' do
  let(:adr) { Rails.root.join('docs/adrs/0008-production-upload-storage.md').read }
  let(:runbook) { Rails.root.join('docs/operations/upload-storage-backup-and-restore.md').read }
  let(:compose) { Rails.root.join('compose.yaml').read }
  let(:deploy) { Rails.root.join('config/deploy.yml').read }

  it 'records the accepted single-node persistent-volume decision' do
    expect(adr).to include('Status: Accepted')
    expect(adr).to include('ReadWriteOnce')
    expect(adr).to include('Recreate')
    expect(adr).to include('Horizontal web scaling is not supported')
    expect(adr).to include('Object storage')
  end

  it 'documents coordinated database and blob backups with retention' do
    expect(runbook).to include('active_storage_attachments')
    expect(runbook).to include('active_storage_blobs')
    expect(runbook).to include('/app/storage')
    expect(runbook).to include('35 daily backups')
    expect(runbook).to include('12 monthly backups')
  end

  it 'documents and exposes the restored-attachment smoke check' do
    expect(runbook).to include('task prod:verify-storage-restore')
    expect(runbook).to include('ATTACHMENT_ID=')
    expect(runbook).to include('isolated')
  end

  it 'keeps production compose storage selection and mount explicit' do
    expect(compose).to include('ACTIVE_STORAGE_SERVICE: ${ACTIVE_STORAGE_SERVICE:-persistent}')
    expect(compose).to include('ACTIVE_STORAGE_ROOT: ${ACTIVE_STORAGE_ROOT:-/app/storage}')
    expect(compose).to include('medtracker_prod_storage:/app/storage')
  end

  it 'aligns the deployment template with the production storage contract' do
    expect(deploy).to include('ACTIVE_STORAGE_SERVICE: persistent')
    expect(deploy).to include('ACTIVE_STORAGE_ROOT: /app/storage')
    expect(deploy).to include('med_tracker_storage:/app/storage')
  end

  it 'creates the production storage root as the unprivileged runtime user' do
    dockerfile = Rails.root.join('Dockerfile').read
    app_stage = dockerfile.split("FROM base AS app\n", 2).last

    expect(app_stage).to include("USER ruby\n\nRUN mkdir -p /app/storage")
  end

  it 'uses the validated production service while preserving local test services' do
    production = Rails.root.join('config/environments/production.rb').read
    storage = Rails.root.join('config/storage.yml').read

    expect(production).to include('ProductionStorage.resolve')
    expect(storage).to include("persistent:\n  service: Disk")
    expect(storage).to include("test:\n  service: Disk")
    expect(storage).to include("local:\n  service: Disk")
  end
end

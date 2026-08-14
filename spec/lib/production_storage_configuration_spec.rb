# frozen_string_literal: true

require 'rails_helper'

RSpec.describe ProductionStorage, '.configuration' do
  let(:compose) { Rails.root.join('compose.yaml').read }
  let(:deploy) { Rails.root.join('config/deploy.yml').read }

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

    expect(app_stage).to include('RUN mkdir -p /app/storage')

    before_storage_setup = app_stage.split('RUN mkdir -p /app/storage', 2).first
    expect(before_storage_setup.scan(/^USER \S+$/).last).to eq('USER ruby')
  end

  it 'uses the validated production service while preserving local test services' do
    production = Rails.root.join('config/environments/production.rb').read
    storage = Rails.root.join('config/storage.yml').read

    expect(production).to include('ProductionStorage.resolve')
    expect(storage).to include(
      "persistent:\n  service: Disk", "s3:\n  service: S3",
      "persistent_with_s3_mirror:\n  service: Mirror",
      "s3_with_persistent_mirror:\n  service: Mirror",
      "test:\n  service: Disk", "local:\n  service: Disk"
    )
  end
end

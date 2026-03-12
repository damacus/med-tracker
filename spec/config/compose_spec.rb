# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'compose.yaml' do
  let(:compose_config) do
    YAML.safe_load(Rails.root.join('compose.yaml').read, aliases: true)
  end

  it 'isolates public assets in development web container' do
    expect(compose_config.dig('services', 'web-dev', 'tmpfs')).to include('/app/public/assets:uid=1000,gid=1000')
  end

  it 'isolates public assets in test web container' do
    expect(compose_config.dig('services', 'web-test', 'tmpfs')).to include('/app/public/assets:uid=1000,gid=1000')
  end
end

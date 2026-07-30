# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Observability::RequestCompletionSubscriber do
  it 'uses the first-party request subscriber without Lograge' do
    expect(Rails.root.join('config/initializers/lograge.rb')).not_to exist
    expect(Rails.root.join('Gemfile').read).not_to match(/gem ['"]lograge['"]/)
    expect(Rails.root.join('Gemfile.lock').read).not_to match(/^\s+lograge\b/)
  end

  it 'disables privacy-unsafe Thruster access records in the final image' do
    expect(Rails.root.join('Dockerfile').read).to include('THRUSTER_LOG_REQUESTS=false')
  end
end

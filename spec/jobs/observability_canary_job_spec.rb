# frozen_string_literal: true

require 'rails_helper'

RSpec.describe ObservabilityCanaryJob do
  it 'emits a job-stage canary without domain arguments' do
    allow(Observability::DeployedCanary).to receive(:emit)

    described_class.perform_now

    expect(Observability::DeployedCanary).to have_received(:emit).with(kind: :job)
  end
end

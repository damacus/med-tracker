# frozen_string_literal: true

require 'rails_helper'
require 'rake'

RSpec.describe Rake::Task do
  let(:task) { described_class['canary:demo_reset'] }
  let(:runner) { instance_double(DemoReset::Runner) }

  before do
    Rails.application.load_tasks unless described_class.task_defined?('canary:demo_reset')
    task.reenable
    allow(DemoReset::Runner).to receive(:new).and_return(runner)
  end

  it 'reports a safe successful command result' do
    allow(runner).to receive(:call).and_return(outcome: 'succeeded', baseline: DemoBaseline::IDENTIFIER)

    expect { task.invoke }.to output(
      "{\"event_type\":\"canary.demo_reset\",\"outcome\":\"succeeded\",\"baseline\":\"weekly-demo-v1\"}\n"
    ).to_stdout
  end

  [
    DemoReset::UnsafeTargetError,
    DemoBaseline::Loader::InvalidBaselineError,
    DemoReset::StorageCleanupError,
    DemoReset::VerificationError,
    ActiveRecord::StatementInvalid
  ].each do |error_class|
    it "exits unsuccessfully and safely for #{error_class}" do
      unsafe_detail = 'postgres://credential@production/health-record'
      allow(runner).to receive(:call).and_raise(error_class, unsafe_detail)

      expect { task.invoke }.to raise_error(SystemExit)
        .and output(satisfy { |value| value.include?('"outcome":"failed"') && value.exclude?(unsafe_detail) }).to_stderr
    end
  end
end

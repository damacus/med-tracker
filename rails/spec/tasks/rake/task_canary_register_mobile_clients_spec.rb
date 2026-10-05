require 'rails_helper'
require 'rake'

RSpec.describe Rake::Task do
  let(:task) { described_class['canary:register_mobile_clients'] }
  let(:runner) { instance_double(Canary::MobileClientRegistration) }

  before do
    Rails.application.load_tasks unless described_class.task_defined?('canary:register_mobile_clients')
    task.reenable
    allow(Canary::MobileClientRegistration).to receive(:new).and_return(runner)
  end

  it 'reports a successful registration' do
    allow(runner).to receive(:call).and_return(outcome: 'succeeded', clients: 4)

    expect { task.invoke }.to output(
      "{\"event_type\":\"canary.register_mobile_clients\",\"outcome\":\"succeeded\",\"clients\":4}\n"
    ).to_stdout
  end

  [DemoReset::UnsafeTargetError, DemoBaseline::PublicMobileClients::MismatchedClientError].each do |error_class|
    it "fails safely for #{error_class}" do
      allow(runner).to receive(:call).and_raise(error_class, 'private target value')

      expect { task.invoke }.to raise_error(SystemExit)
        .and output(satisfy { |value| value.include?('"outcome":"failed"') && value.exclude?('private target value') })
        .to_stderr
    end
  end
end

# frozen_string_literal: true

require 'rails_helper'

RSpec.describe DemoReset::Runner do
  subject(:runner) { described_class.new(**dependencies) }

  let(:reporter) { instance_double(DemoReset::Reporter, stage: nil) }
  let(:dependencies) do
    {
      preflight: callable(outcome: 'passed'),
      primary_reset: callable(baseline: DemoBaseline::IDENTIFIER),
      auxiliary_reset: callable(queue: 2, cache: 1, cable: 1),
      storage_cleaner: callable(objects_removed: 3),
      verifier: callable(baseline: DemoBaseline::IDENTIFIER, storage_objects: 0),
      reporter:
    }
  end

  it 'runs the guarded reset stages in order and returns a safe verified result', :aggregate_failures do
    result = runner.call

    expect(result).to eq(
      outcome: 'succeeded',
      baseline: DemoBaseline::IDENTIFIER,
      primary: { baseline: DemoBaseline::IDENTIFIER },
      auxiliary: { queue: 2, cache: 1, cable: 1 },
      storage: { objects_removed: 3 },
      verification: { baseline: DemoBaseline::IDENTIFIER, storage_objects: 0 }
    )
    expect(reporter).to have_received(:stage).with('preflight', 'succeeded')
    expect(reporter).to have_received(:stage).with('verification', 'succeeded')
  end

  %i[preflight primary_reset auxiliary_reset storage_cleaner verifier].each do |failed_stage|
    it "fails immediately when #{failed_stage} fails" do
      allow(dependencies.fetch(failed_stage)).to receive(:call).and_raise(DemoReset::Error, 'safe_failure')

      expect { runner.call }.to raise_error(DemoReset::Error, 'safe_failure')
      expected_stage = { primary_reset: 'primary', auxiliary_reset: 'auxiliary', verifier: 'verification' }
                       .fetch(failed_stage, failed_stage.to_s)
      expect(reporter).to have_received(:stage).with(expected_stage, 'failed')
    end
  end

  def callable(result)
    instance_double(Proc, call: result)
  end
end

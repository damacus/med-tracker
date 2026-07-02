# frozen_string_literal: true

require 'rails_helper'

RSpec.describe ApplicationJob do
  let(:job_class) do
    Class.new(described_class) do
      def perform
        raise 'job failure'
      end
    end
  end

  it 'records job exceptions on the current trace span before reraising' do
    allow(Otel::ExceptionRecorder).to receive(:record)

    expect { job_class.perform_now }.to raise_error(RuntimeError, 'job failure')
    expect(Otel::ExceptionRecorder).to have_received(:record)
      .with(instance_of(RuntimeError), source: 'job')
  end
end

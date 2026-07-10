# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Audit::ObjectLock::Runner do
  let(:adapter) { instance_double(Audit::ObjectLock::S3Adapter, validate!: true) }
  let(:signer) { instance_double(Audit::CheckpointSigner) }
  let(:delivery_exporter) { instance_double(Audit::ObjectLock::DeliveryExporter) }
  let(:records) do
    { entry: instance_double(AuditLedgerEntry), delivery: instance_double(AuditExportDelivery) }
  end
  let(:runner) do
    described_class.new(
      adapter:, signer:, delivery_exporter:,
      checkpoint_entries: -> { [records[:entry]] }, pending_deliveries: -> { [records[:delivery]] }
    )
  end

  it 'validates storage before signing checkpoints and draining the outbox' do
    allow(signer).to receive(:sign).with(records[:entry])
    allow(delivery_exporter).to receive(:deliver).with(records[:delivery])

    runner.startup!
    runner.run_once

    expect(adapter).to have_received(:validate!).ordered
    expect(signer).to have_received(:sign).with(records[:entry]).ordered
    expect(delivery_exporter).to have_received(:deliver).with(records[:delivery]).ordered
  end

  it 'continues when a checkpoint was already signed by another exporter' do
    allow(signer).to receive(:sign).and_raise(Audit::CheckpointSigner::AlreadySigned)
    allow(delivery_exporter).to receive(:deliver)

    expect { runner.run_once }.not_to raise_error
    expect(delivery_exporter).to have_received(:deliver).with(records[:delivery])
  end

  it 'revalidates Object Lock configuration on the scheduled interval' do
    now = 100
    scheduled_runner = described_class.new(
      adapter:, signer:, delivery_exporter:, checkpoint_entries: -> { [] }, pending_deliveries: -> { [] },
      clock: -> { now }, validation_interval: 300
    )

    scheduled_runner.startup!
    now = 401
    scheduled_runner.run_once

    expect(adapter).to have_received(:validate!).twice
  end
end

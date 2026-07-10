# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Storage::RestoreVerifier do
  subject(:verification) { described_class.call(attachment_id: attachment.id) }

  let(:person) { create(:person) }
  let(:attachment) { person.avatar.attachment }

  before do
    person.avatar.attach(io: StringIO.new('restored avatar'), filename: 'avatar.png', content_type: 'image/png')
  end

  after do
    person.avatar.purge
  end

  it 'verifies that the restored blob exists and matches its stored checksum' do
    result = verification

    expect(result.attachment_id).to eq(attachment.id)
    expect(result.blob_id).to eq(attachment.blob_id)
    expect(result.byte_size).to eq('restored avatar'.bytesize)
  end

  it 'rejects a database attachment whose stored object is missing' do
    allow(attachment.blob.service).to receive(:exist?).with(attachment.blob.key).and_return(false)

    expect { verification }
      .to raise_error(described_class::VerificationError, /stored object is missing/)
  end

  it 'rejects a restored object that fails the Active Storage integrity check' do
    allow(ActiveStorage::Attachment).to receive(:find_by).with(id: attachment.id).and_return(attachment)
    allow(attachment.blob).to receive(:open).and_raise(ActiveStorage::IntegrityError)

    expect { verification }
      .to raise_error(described_class::VerificationError, /checksum/)
  end

  it 'requires an attachment from the restored database' do
    expect { described_class.call(attachment_id: 'missing') }
      .to raise_error(described_class::VerificationError, /attachment/)
  end
end

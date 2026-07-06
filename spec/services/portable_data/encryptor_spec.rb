# frozen_string_literal: true

require 'rails_helper'

RSpec.describe PortableData::Encryptor do
  let(:passphrase) { 'correct horse battery staple' }
  let(:payload) { { format: 'medtracker.portable.v1', records: {} } }
  let(:envelope) { described_class.encrypt(payload, passphrase: passphrase) }

  it 'round-trips portable payloads' do
    expect(described_class.decrypt(envelope, passphrase: passphrase)).to eq(payload.deep_stringify_keys)
  end

  it 'requires a passphrase for encryption' do
    expect do
      described_class.encrypt(payload, passphrase: '')
    end.to raise_error(described_class::Error, 'Passphrase is required')
  end

  it 'requires a passphrase for decryption' do
    expect do
      described_class.decrypt(envelope, passphrase: nil)
    end.to raise_error(described_class::Error, 'Passphrase is required')
  end

  it 'rejects unsupported envelope formats' do
    unsupported = envelope.merge(format: 'medtracker.portable.encrypted.v0')

    expect do
      described_class.decrypt(unsupported, passphrase: passphrase)
    end.to raise_error(described_class::Error, 'Unsupported portable data envelope')
  end

  it 'rejects unsupported envelope ciphers' do
    unsupported = envelope.merge(cipher: 'aes-128-gcm')

    expect do
      described_class.decrypt(unsupported, passphrase: passphrase)
    end.to raise_error(described_class::Error, 'Unsupported portable data envelope')
  end

  it 'rejects unsupported envelope key derivation functions' do
    unsupported = envelope.merge(kdf: 'argon2id')

    expect do
      described_class.decrypt(unsupported, passphrase: passphrase)
    end.to raise_error(described_class::Error, 'Unsupported portable data envelope')
  end
end

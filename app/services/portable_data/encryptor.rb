# frozen_string_literal: true

module PortableData
  class Encryptor
    class Error < StandardError; end

    FORMAT = 'medtracker.portable.encrypted.v1'
    CIPHER = 'aes-256-gcm'

    def self.encrypt(payload, passphrase:)
      new(passphrase).encrypt(payload)
    end

    def self.decrypt(envelope, passphrase:)
      new(passphrase).decrypt(envelope)
    end

    def initialize(passphrase)
      @passphrase = passphrase.to_s
    end

    def encrypt(payload)
      validate_passphrase!
      salt = SecureRandom.hex(32)
      plaintext = JSON.generate(payload.as_json)

      {
        format: FORMAT,
        encrypted_at: Time.current.iso8601,
        cipher: CIPHER,
        kdf: 'pbkdf2_sha256',
        salt: salt,
        checksum: Digest::SHA256.hexdigest(plaintext),
        ciphertext: encryptor(salt).encrypt_and_sign(plaintext)
      }
    end

    def decrypt(envelope)
      validate_passphrase!
      data = envelope.to_h.with_indifferent_access
      raise Error, 'Unsupported portable data envelope' unless data[:format] == FORMAT

      plaintext = encryptor(data.fetch(:salt)).decrypt_and_verify(data.fetch(:ciphertext))
      raise Error, 'Portable data checksum mismatch' unless Digest::SHA256.hexdigest(plaintext) == data[:checksum]

      JSON.parse(plaintext)
    rescue ActiveSupport::MessageEncryptor::InvalidMessage, KeyError, JSON::ParserError
      raise Error, 'Portable data could not be decrypted'
    end

    private

    attr_reader :passphrase

    def validate_passphrase!
      raise Error, 'Passphrase is required' if passphrase.blank?
    end

    def encryptor(salt)
      ActiveSupport::MessageEncryptor.new(key_for(salt), cipher: CIPHER)
    end

    def key_for(salt)
      ActiveSupport::KeyGenerator.new(
        passphrase,
        hash_digest_class: OpenSSL::Digest::SHA256
      ).generate_key(salt, ActiveSupport::MessageEncryptor.key_len(CIPHER))
    end
  end
end

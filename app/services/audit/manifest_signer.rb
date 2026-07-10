# frozen_string_literal: true

require 'base64'
require 'json'
require 'openssl'

module Audit
  class ManifestSigner
    def self.canonical_json(value)
      JSON.generate(deep_sort(value))
    end

    def self.deep_sort(value)
      case value
      when Hash then value.to_h { |key, child| [key.to_s, deep_sort(child)] }.sort.to_h
      when Array then value.map { |child| deep_sort(child) }
      else value
      end
    end

    def initialize(key_id:, private_key_pem:)
      @key_id = key_id
      @private_key = OpenSSL::PKey.read(private_key_pem)
      raise ArgumentError, 'manifest signing key must be Ed25519' unless private_key.oid == 'ED25519'
    end

    def sign(manifest)
      signature = private_key.sign(nil, self.class.canonical_json(manifest))
      manifest.merge(
        signing: {
          key_id:, algorithm: 'ed25519', public_key: Base64.strict_encode64(private_key.public_to_der),
          signature: Base64.strict_encode64(signature)
        }
      )
    end

    private

    attr_reader :key_id, :private_key
  end
end

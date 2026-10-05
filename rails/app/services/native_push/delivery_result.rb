# frozen_string_literal: true

module NativePush
  DeliveryResult = Data.define(:status, :provider_status, :provider_error) do
    def self.delivered(provider_status: 200)
      new(status: :delivered, provider_status: provider_status, provider_error: nil)
    end

    def self.skipped(provider_error:)
      new(status: :skipped, provider_status: nil, provider_error: provider_error)
    end

    def self.failed(provider_status:, provider_error:)
      new(status: :failed, provider_status: provider_status, provider_error: provider_error)
    end

    def self.unregistered(provider_status: nil, provider_error: nil)
      new(status: :unregistered, provider_status: provider_status, provider_error: provider_error)
    end

    def unregistered?
      status == :unregistered
    end
  end
end

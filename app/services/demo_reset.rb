# frozen_string_literal: true

module DemoReset
  class Error < StandardError; end
  class UnsafeTargetError < Error; end
  class StorageCleanupError < Error; end
  class VerificationError < Error; end
end

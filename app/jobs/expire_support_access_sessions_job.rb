# frozen_string_literal: true

class ExpireSupportAccessSessionsJob < ApplicationJob
  def perform
    SupportAccessSessions::ExpiryProcessor.call
  end
end

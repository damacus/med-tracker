# frozen_string_literal: true

class ExpireHouseholdExportsJob < ApplicationJob
  def perform
    Households::ExportExpiryProcessor.call
  end
end

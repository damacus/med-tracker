# frozen_string_literal: true

class NhsDmdBarcode < ApplicationRecord
  validates :gtin, presence: true, uniqueness: true
  validates :code, presence: true
  validates :display, presence: true
  validates :system, presence: true

  after_commit :expire_cache

  def self.normalize_gtin(value)
    value.to_s.gsub(/\D/, '')
  end

  private

  def expire_cache
    NhsDmd::BarcodeLookup.expire(gtin)
  end
end

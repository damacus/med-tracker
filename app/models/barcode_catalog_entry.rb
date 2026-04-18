# frozen_string_literal: true

class BarcodeCatalogEntry < ApplicationRecord
  validates :gtin, presence: true
  validates :display, presence: true
  validates :source, presence: true
  validates :gtin, uniqueness: { scope: :source }

  def self.normalize_gtin(value)
    value.to_s.gsub(/\D/, '')
  end
end

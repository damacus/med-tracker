# frozen_string_literal: true

class NhsDmdTradeFamilyGroup < ApplicationRecord
  has_many :trade_families, class_name: 'NhsDmdTradeFamily', dependent: :restrict_with_exception

  validates :code, :name, presence: true
  validates :code, uniqueness: true
end

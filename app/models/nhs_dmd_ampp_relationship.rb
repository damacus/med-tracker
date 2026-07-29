# frozen_string_literal: true

class NhsDmdAmppRelationship < ApplicationRecord
  validates :ampp_code, :amp_code, presence: true
  validates :ampp_code, uniqueness: true
end

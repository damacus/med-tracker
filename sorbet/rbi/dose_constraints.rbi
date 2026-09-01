# typed: true

class MedicationTake
  sig { returns(Time) }
  def taken_at; end
end

module Prism
  module LexCompat
    class Result; end
  end
end

class Time
  class << self
    sig { returns(Time) }
    def current; end
  end
end

# typed: true

module ActiveSupport
  class Duration; end

  class TimeWithZone
    sig { returns(T::Range[ActiveSupport::TimeWithZone]) }
    def all_day; end

    sig { returns(T::Range[ActiveSupport::TimeWithZone]) }
    def all_week; end

    sig { returns(T::Range[ActiveSupport::TimeWithZone]) }
    def all_month; end

    sig { returns(ActiveSupport::TimeWithZone) }
    def end_of_day; end

    sig { returns(ActiveSupport::TimeWithZone) }
    def end_of_week; end

    sig { returns(ActiveSupport::TimeWithZone) }
    def end_of_month; end

    sig { params(value: T.any(Numeric, ActiveSupport::Duration)).returns(ActiveSupport::TimeWithZone) }
    def +(value); end
  end
end

class Integer
  sig { returns(ActiveSupport::Duration) }
  def second; end

  sig { returns(ActiveSupport::Duration) }
  def day; end

  sig { returns(ActiveSupport::Duration) }
  def week; end

  sig { returns(ActiveSupport::Duration) }
  def month; end
end

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
  sig { returns(T::Range[Time]) }
  def all_day; end

  sig { returns(T::Range[Time]) }
  def all_week; end

  sig { returns(T::Range[Time]) }
  def all_month; end

  sig { returns(Time) }
  def end_of_day; end

  sig { returns(Time) }
  def end_of_week; end

  sig { returns(Time) }
  def end_of_month; end

  sig { params(value: T.any(Numeric, ActiveSupport::Duration)).returns(Time) }
  def +(value); end

  class << self
    sig { returns(Time) }
    def current; end
  end
end

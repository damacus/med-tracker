# frozen_string_literal: true

# typed: true

require 'sorbet-runtime'

class DoseCycle
  extend T::Sig

  CycleInput = T.type_alias { T.nilable(T.any(String, Symbol)) }
  TimeValue = T.type_alias { T.any(Time, ActiveSupport::TimeWithZone) }
  VALID_CYCLES = T.let(%w[daily weekly monthly].freeze, T::Array[String])

  sig { params(value: CycleInput).void }
  def initialize(value)
    str = value.to_s
    @value = T.let(VALID_CYCLES.include?(str) ? str : 'daily', String)
  end

  sig { params(time: TimeValue).returns(T::Range[TimeValue]) }
  def range_for(time)
    case @value
    when 'weekly' then time.all_week
    when 'monthly' then time.all_month
    else time.all_day
    end
  end

  sig do
    type_parameters(:TimeType)
      .params(time: T.all(T.type_parameter(:TimeType), TimeValue))
      .returns(T.type_parameter(:TimeType))
  end
  def next_reset_time(time)
    result = case @value
             when 'weekly' then time.end_of_week + 1.second
             when 'monthly' then time.end_of_month + 1.second
             else time.end_of_day + 1.second
             end
    T.cast(result, T.type_parameter(:TimeType))
  end

  sig { returns(ActiveSupport::Duration) }
  def period
    case @value
    when 'weekly' then 1.week
    when 'monthly' then 1.month
    else 1.day
    end
  end

  sig { returns(String) }
  def to_s = @value
end

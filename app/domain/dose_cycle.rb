# frozen_string_literal: true

class DoseCycle
  VALID_CYCLES = %w[daily weekly monthly].freeze

  def initialize(value)
    str = value.to_s
    @value = VALID_CYCLES.include?(str) ? str : 'daily'
  end

  def range_for(time)
    case @value
    when 'weekly' then time.all_week
    when 'monthly' then time.all_month
    else time.all_day
    end
  end

  def next_reset_time(time)
    case @value
    when 'weekly' then time.end_of_week + 1.second
    when 'monthly' then time.end_of_month + 1.second
    else time.end_of_day + 1.second
    end
  end

  def period
    case @value
    when 'weekly' then 7.days
    when 'monthly' then 30.days
    else 1.day
    end
  end

  def to_s = @value
end

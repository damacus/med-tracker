# frozen_string_literal: true

FactoryBot.define do
  factory :notification_preference do
    person
    enabled { true }
    morning_time { '08:00:00' }
    afternoon_time { '14:00:00' }
    evening_time { '18:00:00' }
    night_time { '22:00:00' }
  end
end

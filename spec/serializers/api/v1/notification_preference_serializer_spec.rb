# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Api::V1::NotificationPreferenceSerializer do
  it 'serialises the preference with HH:MM:SS period times' do
    preference = create(:notification_preference, enabled: true, morning_time: '08:30', night_time: nil)
    expect(described_class.new(preference).as_json).to include(
      id: preference.id, person_id: preference.person_id, enabled: true,
      updated_at: preference.updated_at.iso8601, morning_time: '08:30:00', night_time: nil
    )
  end

  it 'includes all four period time keys' do
    preference = create(:notification_preference,
                        morning_time: '08:00', afternoon_time: '14:00',
                        evening_time: '18:00', night_time: '22:00')
    json = described_class.new(preference).as_json
    expect(json).to include(
      morning_time: '08:00:00',
      afternoon_time: '14:00:00',
      evening_time: '18:00:00',
      night_time: '22:00:00'
    )
  end

  it 'formats nil period times as nil' do
    preference = create(:notification_preference,
                        morning_time: nil, afternoon_time: nil,
                        evening_time: nil, night_time: nil)
    json = described_class.new(preference).as_json
    expect(json[:morning_time]).to be_nil
    expect(json[:afternoon_time]).to be_nil
    expect(json[:evening_time]).to be_nil
    expect(json[:night_time]).to be_nil
  end
end

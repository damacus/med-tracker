# frozen_string_literal: true

require 'rails_helper'

RSpec.describe ExpireHouseholdExportsJob do
  it 'delegates scheduled expiry processing' do
    allow(Households::ExportExpiryProcessor).to receive(:call).and_return(2)

    expect(described_class.perform_now).to eq(2)
  end

  it 'runs hourly in production' do
    schedule = YAML.safe_load_file(Rails.root.join('config/recurring.yml')).dig(
      'production', 'expire_household_exports'
    )

    expect(schedule).to include(
      'class' => described_class.name, 'schedule' => 'every hour', 'queue' => 'default'
    )
  end
end

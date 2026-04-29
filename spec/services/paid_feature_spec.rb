# frozen_string_literal: true

require 'rails_helper'

RSpec.describe PaidFeature do
  fixtures :accounts, :people, :users

  describe '.enabled?' do
    it 'keeps AI medication help disabled by default' do
      allow(ENV).to receive(:fetch).with('MEDTRACKER_AI_MEDICATION_HELP_ENABLED', 'false').and_return('false')

      expect(described_class.enabled?(:ai_medication_help, user: users(:admin))).to be(false)
    end

    it 'enables AI medication help when the environment flag is true' do
      allow(ENV).to receive(:fetch).with('MEDTRACKER_AI_MEDICATION_HELP_ENABLED', 'false').and_return('true')

      expect(described_class.enabled?(:ai_medication_help, user: users(:admin))).to be(true)
    end

    it 'disables unknown paid features' do
      expect(described_class.enabled?(:unknown, user: users(:admin))).to be(false)
    end
  end
end

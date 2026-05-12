# frozen_string_literal: true

require 'rails_helper'

RSpec.describe User do
  fixtures :accounts, :people, :users

  describe '#gravatar_enabled?' do
    it 'defaults to false' do
      expect(users(:damacus).gravatar_enabled?).to be(false)
    end

    it 'normalizes truthy preference values' do
      user = users(:damacus)

      user.gravatar_enabled = '1'

      expect(user.gravatar_enabled?).to be(true)
    end
  end
end

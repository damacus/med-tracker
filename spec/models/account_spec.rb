# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Account do
  describe 'associations' do
    it { is_expected.to have_one(:person).dependent(:nullify) }
  end

  describe 'enum' do
    it { is_expected.to define_enum_for(:status).with_values(unverified: 1, verified: 2, closed: 3) }
  end
end

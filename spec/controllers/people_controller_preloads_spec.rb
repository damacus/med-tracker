# frozen_string_literal: true

require 'rails_helper'

RSpec.describe PeopleController do
  it 'keeps index preloads in sync with PersonCard requirements' do
    expect(described_class::INDEX_PRELOADS).to include(*Components::People::PersonCard::REQUIRED_PRELOADS)
  end
end

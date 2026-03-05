# frozen_string_literal: true

module ComponentPreloadContracts
  def expect_required_preloads(component_class, expected_preloads)
    expect(component_class::REQUIRED_PRELOADS).to eq(expected_preloads)
  end
end

RSpec.configure do |config|
  config.include ComponentPreloadContracts, type: :component
end

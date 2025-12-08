# frozen_string_literal: true

module Components
  # Shared helper methods for form components
  # Include this module in components that need consistent form styling
  module FormHelpers
    extend ActiveSupport::Concern

    # Standard styling for native select elements to match RubyUI components
    # Use this when RubyUI::Select is not compatible with Capybara tests
    def select_classes
      'flex h-9 w-full items-center justify-between rounded-md border border-input ' \
        'bg-transparent px-3 py-2 text-sm shadow-sm ring-offset-background ' \
        'focus:outline-none focus:ring-1 focus:ring-ring disabled:cursor-not-allowed disabled:opacity-50'
    end
  end
end

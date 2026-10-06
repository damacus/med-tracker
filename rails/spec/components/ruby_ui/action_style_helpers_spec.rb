# frozen_string_literal: true

require 'rails_helper'

RSpec.describe RubyUI::ActionStyleHelpers do
  subject(:helper) { helper_class.new(size:, icon:) }

  let(:helper_class) do
    Class.new do
      include RubyUI::ActionStyleHelpers

      def initialize(size:, icon:)
        @size = size
        @icon = icon
      end

      public :base_classes, :size_classes
    end
  end

  describe '#base_classes' do
    let(:size) { :md }
    let(:icon) { false }
    let(:expected_classes) do
      [
        'whitespace-nowrap inline-flex items-center justify-center rounded-shape-xl font-medium transition-colors',
        'disabled:pointer-events-none disabled:opacity-50',
        'focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring',
        'aria-disabled:pointer-events-none aria-disabled:opacity-50 aria-disabled:cursor-not-allowed'
      ]
    end

    it 'returns the shared interactive element classes' do
      expect(helper.base_classes).to eq(expected_classes)
    end
  end

  describe '#size_classes' do
    context 'when icon is false and size is md' do
      let(:icon) { false }
      let(:size) { :md }

      it 'returns the shared text button spacing' do
        expect(helper.size_classes).to eq('px-4 py-2 min-h-[44px] text-sm')
      end
    end

    context 'when icon is true and size is lg' do
      let(:icon) { true }
      let(:size) { :lg }

      it 'returns the shared icon sizing' do
        expect(helper.size_classes).to eq('h-11 w-11 min-h-11 min-w-11')
      end
    end
  end
end

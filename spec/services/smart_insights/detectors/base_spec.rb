# frozen_string_literal: true

require 'rails_helper'

RSpec.describe SmartInsights::Detectors::Base do
  let(:context) { instance_double(SmartInsights::Context) }

  subject(:detector) { described_class.new(context) }

  describe '#initialize / #context' do
    it 'stores the context' do
      expect(detector.context).to eq(context)
    end
  end

  describe '#call' do
    it 'raises NotImplementedError with an informative message' do
      expect { detector.call }.to raise_error(
        NotImplementedError,
        /SmartInsights::Detectors::Base has not implemented method 'call'/
      )
    end

    it 'includes the subclass name in the error when called on a subclass' do
      subclass = Class.new(described_class)
      stub_const('MyCustomDetector', subclass)
      expect { subclass.new(context).call }.to raise_error(
        NotImplementedError,
        /MyCustomDetector has not implemented method 'call'/
      )
    end
  end

  describe 'private #insight helper' do
    # Exercise the private insight factory through a thin subclass
    let(:subclass) do
      Class.new(described_class) do
        def build_insight(**attrs)
          insight(**attrs)
        end
      end
    end

    subject(:instance) { subclass.new(context) }

    it 'defaults cta_path to nil when not provided' do
      result = instance.build_insight(
        key: :test,
        family: :adherence,
        severity: :positive,
        title: 'T',
        summary: 'S',
        detail: 'D',
        metric_label: 'ML',
        metric_value: 'MV'
      )
      expect(result.cta_path).to be_nil
    end

    it 'preserves cta_path when explicitly supplied' do
      result = instance.build_insight(
        key: :test,
        family: :adherence,
        severity: :positive,
        title: 'T',
        summary: 'S',
        detail: 'D',
        metric_label: 'ML',
        metric_value: 'MV',
        cta_path: '/dashboard'
      )
      expect(result.cta_path).to eq('/dashboard')
    end

    it 'returns a SmartInsights::Insight value object' do
      result = instance.build_insight(
        key: :test,
        family: :adherence,
        severity: :positive,
        title: 'T',
        summary: 'S',
        detail: 'D',
        metric_label: 'ML',
        metric_value: 'MV'
      )
      expect(result).to be_a(SmartInsights::Insight)
    end
  end
end

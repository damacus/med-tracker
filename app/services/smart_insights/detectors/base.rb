# frozen_string_literal: true

module SmartInsights
  module Detectors
    class Base
      attr_reader :context

      def initialize(context)
        @context = context
      end

      def call
        raise NotImplementedError, "#{self.class} has not implemented method '#{__method__}'"
      end

      private

      def insight(**attributes)
        attributes[:cta_path] = nil unless attributes.key?(:cta_path)
        Insight.new(**attributes)
      end
    end
  end
end

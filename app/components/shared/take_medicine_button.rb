# frozen_string_literal: true

module Components
  module Shared
    class TakeMedicineButton < Components::Base
      include Phlex::Rails::Helpers::FormWith

      attr_reader :takeable, :take_url

      def initialize(takeable:, take_url:)
        @takeable = takeable
        @take_url = take_url
        super()
      end

      def view_template
        if takeable.can_administer?
          render_enabled_button
        else
          render_disabled_button
        end
      end

      private

      def render_enabled_button
        form_with(
          url: take_url,
          method: :post,
          class: 'inline-block',
          data: { controller: 'optimistic-take', action: 'submit->optimistic-take#submit' }
        ) do
          Button(
            type: :submit,
            variant: :primary,
            size: :md,
            class: 'inline-flex items-center gap-1 min-w-[80px]',
            data: { optimistic_take_target: 'button' }
          ) do
            plain '💊 Take'
          end
        end
      end

      def render_disabled_button
        reason = takeable.administration_blocked_reason
        label = reason == :out_of_stock ? '💊 Out of Stock' : '💊 Take'
        Button(variant: :secondary, size: :md, disabled: true) { label }
      end
    end
  end
end

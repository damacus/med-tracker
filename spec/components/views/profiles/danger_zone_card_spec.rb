# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Views::Profiles::DangerZoneCard, type: :component do
  it 'uses token-driven destructive styling for the danger zone card' do
    stub_const('Views::Profiles::CloseAccountDialog', Class.new(Components::Base) do
      def view_template
        plain 'Close account'
      end
    end)

    rendered = render_inline(described_class.new)
    html = rendered.to_html

    banned_classes = ['bg-[linear-gradient', 'dark:bg-[linear-gradient', 'rounded-[2rem]',
                      'shadow-[0_18px_45px_-32px_rgba']

    expect(banned_classes.none? { |class_name| html.include?(class_name) }).to be(true)
  end
end

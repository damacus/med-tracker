# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Views::Profiles::VersionInfo, type: :component do
  it 'uses token-driven shells for version info' do
    rendered = render_inline(described_class.new)
    html = rendered.to_html

    banned_classes = ['rounded-[2rem]', 'bg-card/95', 'shadow-[0_18px_45px_-32px_rgba']

    expect(banned_classes.none? { |class_name| html.include?(class_name) }).to be(true)
  end
end

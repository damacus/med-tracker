# frozen_string_literal: true

require "rails_helper"

RSpec.describe Components::Icons::Calendar, type: :component do
  it "merges caller attributes with the shared icon defaults" do
    rendered = render_inline(
      described_class.new(class: "mt-0.5 text-on-surface-variant", aria_hidden: "true", size: 20)
    )
    svg = rendered.at_css("svg")

    expect(svg["class"].split).to(include("lucide", "lucide-calendar", "mt-0.5", "text-on-surface-variant"))
    expect(svg["aria-hidden"]).to(eq("true"))
    expect(svg["width"]).to(eq("20"))
  end
end

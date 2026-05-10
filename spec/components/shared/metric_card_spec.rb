# frozen_string_literal: true

require "rails_helper"

RSpec.describe Components::Shared::MetricCard, type: :component do
  let(:active_schedules_icon_path) do
    [
      "M200-640h560v-80H200v80Zm0 0v-80 80Zm0 560q-33 0-56.5-23.5T120-160v-560q0-33 ",
      "23.5-56.5T200-800h40v-80h80v80h320v-80h80v80h40q33 0 56.5 23.5T840-720v227q-19-9-39-15t-41-9v-43H200v400h252q7 ",
      "22 16.5 42T491-80H200Zm378.5-18.5Q520-157 520-240t58.5-141.5Q637-440 ",
      "720-440t141.5 58.5Q920-323 920-240T861.5-98.5Q803-40 ",
      "720-40T578.5-98.5ZM787-145l28-28-75-75v-112h-40v128l87 87Z"
    ].join
  end

  let(:compliance_icon_path) do
    [
      "M480-80q-139-35-229.5-159.5T160-516v-244l320-120 320 120v200h-80v-145l-240-90-240 ",
      "90v189q0 121 68 220t172 132q26-8 49.5-20.5T576-214l56 56q-33 27-71.5 47T480-80Zm331.5-11.5Q800-103 ",
      "800-120t11.5-28.5Q823-160 840-160t28.5 11.5Q880-137 880-120t-11.5 28.5Q857-80 ",
      "840-80t-28.5-11.5ZM800-240v-240h80v240h-80ZM480-480Zm56.5 ",
      "56.5Q560-447 560-480t-23.5-56.5Q513-560 480-560t-56.5 23.5Q400-513 400-480t23.5 ",
      "56.5Q447-400 480-400t56.5-23.5ZM480-320q-66 ",
      "0-113-47t-47-113q0-66 47-113t113-47q66 0 113 47t47 113q0 22-5.5 42.5T618-398l119 ",
      "118-57 57-120-119q-18 11-38.5 16.5T480-320Z"
    ].join
  end

  it "renders a block link wrapper when href is provided" do
    rendered = render_inline(
      described_class.new(title: "People", value: 5, icon_type: "users", href: "/people")
    )

    link = rendered.at_css("a[href=\"/people\"]")
    expect(link).to(be_present)
    expect(link["class"]).to(include("block"))
    expect(link["class"]).to(include("h-full"))
    expect(link["class"]).not_to(include("h-9"))
  end

  it "renders a non-link wrapper when href is omitted" do
    rendered = render_inline(
      described_class.new(title: "People", value: 5, icon_type: "users")
    )

    expect(rendered.css("a")).to(be_empty)
    expect(rendered.css("div.h-full")).to(be_present)
  end

  it "renders badge text when provided" do
    rendered = render_inline(
      described_class.new(title: "Compliance", value: "85%", icon_type: "check", badge: "Needs review")
    )

    expect(rendered.text).to(include("Needs review"))
  end

  it "applies warning variant classes" do
    rendered = render_inline(
      described_class.new(title: "No Carers", value: 2, icon_type: "activity", variant: :warning)
    )
    html = rendered.to_html

    expect(html).to(include("bg-warning-container"))
    expect(html).to(include("border-warning"))
    expect(html).to(include("text-on-warning-container"))
  end

  it "adds custom data attributes to the value element" do
    rendered = render_inline(
      described_class.new(
        title: "Total Users",
        value: 10,
        icon_type: "users",
        value_data_attr: {metric_value: 10}
      )
    )

    expect(rendered.css("[data-metric-value=\"10\"]")).to(be_present)
  end

  it "renders compact layout classes when requested" do
    rendered = render_inline(
      described_class.new(title: "Next Dose", value: "14:47", icon_type: "clock", layout: :compact)
    )
    html = rendered.to_html
    card = rendered.css("div").find do |node|
      node[:class]&.include?("min-h-[7rem]")
    end

    expect(html).to(include("min-h-[7rem]"))
    expect(html).to(include("p-4"))
    expect(html).to(include("text-2xl"))
    expect(html).not_to(include("md:hover:scale-[1.02]"))
    expect(card[:class]).not_to(include("h-full"))
  end

  it "renders the active schedules icon path" do
    rendered = render_inline(
      described_class.new(title: "Active Schedules", value: 10, icon_type: "active_schedules")
    )

    expect(rendered.at_css("svg")["viewbox"]).to(eq("0 -960 960 960"))
    expect(rendered.at_css("path[d='#{active_schedules_icon_path}']")).to(be_present)
  end

  it "renders the compliance icon path" do
    rendered = render_inline(
      described_class.new(title: "Compliance", value: "85%", icon_type: "compliance")
    )

    expect(rendered.at_css("svg")["viewbox"]).to(eq("0 -960 960 960"))
    expect(rendered.at_css("path[d='#{compliance_icon_path}']")).to(be_present)
  end
end

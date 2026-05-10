# frozen_string_literal: true

require "rails_helper"

RSpec.describe ApplicationController do
  let(:action_class) { Data.define(:controller, :name, :line_number, :body_lines) }
  let(:controllers) do
    [
      MedicationsController,
      PersonMedicationsController,
      SchedulesController
    ]
  end

  it "keeps each public action under ten nonblank body lines" do
    oversized_actions = controllers
      .flat_map { |controller| public_actions_for(controller) }
      .select { |action| action.body_lines > 10 }

    expect(oversized_actions).to(eq([]))
  end

  def public_actions_for(controller)
    lines = controller_source_lines(controller)
    public_method_lines(lines).filter_map do |line, index|
      build_action(controller, line, index, lines)
    end
  end

  def controller_source_lines(controller)
    Rails.root.join("app/controllers", controller.name.underscore.concat(".rb")).readlines
  end

  def public_method_lines(lines)
    private_line = private_line_number(lines)
    lines.each_with_index.take_while { |_line, index| index < private_line }
  end

  def private_line_number(lines)
    lines.index { |line| line.match?(/^\s*private\s*$/) } || lines.length
  end

  def build_action(controller, line, index, lines)
    match = line.match(/^\s*def\s+([a-z_?]+)/)
    return unless match

    action_class.new(
      controller: controller.name,
      name: match[1],
      line_number: index + 1,
      body_lines: action_body_line_count(lines, index)
    )
  end

  def action_body_line_count(lines, start_index)
    body_lines_for(lines, start_index).count { |line| line.strip.present? }
  end

  def body_lines_for(lines, start_index)
    depth = 0
    body_lines = []

    lines[(start_index + 1)..].each do |line|
      depth -= 1 if line.match?(/^\s*end\s*$/)
      break if depth.negative?

      body_lines << line
      stripped_line = line.strip
      depth += 1 if stripped_line.match?(/\A(if|unless|case|begin)\b/) || stripped_line.match?(/\bdo\s*(\|.*\|)?\s*\z/)
    end

    body_lines
  end
end

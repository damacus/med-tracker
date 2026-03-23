require 'phlex'

class TestView < Phlex::HTML
  def template
    button(aria_label: "My Label") do
      "Hello"
    end
  end
end

puts TestView.new.call

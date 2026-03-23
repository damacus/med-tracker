require 'phlex'

class TestPhlex < Phlex::HTML
  def template
    button(aria_label: 'test') { 'hi' }
  end
end

puts TestPhlex.new.call

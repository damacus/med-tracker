require 'bundler/setup'
require 'phlex'

class Test < Phlex::HTML
  def template
    button(aria_label: "test") { "Hello" }
  end
end

puts Test.new.call

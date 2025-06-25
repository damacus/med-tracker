class HomeController < ApplicationController
  def index
    render Components::Home::IndexView.new
  end
end

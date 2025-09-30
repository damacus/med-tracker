# frozen_string_literal: true

class HomeController < ApplicationController
  def index
    render Components::Home::IndexView.new
  end
end

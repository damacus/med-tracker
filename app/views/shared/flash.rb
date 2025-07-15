# frozen_string_literal: true

class Views::Shared::Flash < Views::Base
  def initialize(flash:)
    super()
    @flash = flash
  end

  def view_template
    if @flash[:notice]
      div(class: "alert alert--success") do
        span { @flash[:notice] }
      end
    end

    if @flash[:alert]
      div(class: "alert alert--error") do
        span { @flash[:alert] }
      end
    end
  end
end

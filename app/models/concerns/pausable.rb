# frozen_string_literal: true

module Pausable
  def paused? = !active

  def pause! = update!(active: false)

  def resume! = update!(active: true)
end

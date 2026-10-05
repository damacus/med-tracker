# frozen_string_literal: true

module TurboNativeDetectable
  extend ActiveSupport::Concern

  included do
    layout :resolve_layout
    helper_method :turbo_native_app?
  end

  private

  def turbo_native_app?
    request.user_agent.to_s.include?('Turbo Native')
  end

  def resolve_layout
    turbo_native_app? ? 'native' : 'application'
  end
end

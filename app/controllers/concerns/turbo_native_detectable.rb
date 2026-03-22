# frozen_string_literal: true

# Detects requests originating from a Turbo Native iOS or Android client.
#
# Turbo Native apps append "Turbo Native iOS" or "Turbo Native Android" to
# their User-Agent string. When detected, the controller switches to the
# stripped-down "native" layout that omits the sidebar/navigation rendered
# by the web layout (the native shell provides its own chrome).
module TurboNativeDetectable
  extend ActiveSupport::Concern

  included do
    before_action :set_native_layout
    helper_method :turbo_native_app?
  end

  private

  def turbo_native_app?
    request.user_agent.to_s.include?("Turbo Native")
  end

  def set_native_layout
    self.class.layout("native") if turbo_native_app?
  end
end

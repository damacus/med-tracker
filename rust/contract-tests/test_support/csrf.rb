require 'active_support'

module ContractCsrf
  def protect_against_forgery?
    true
  end
end

ActiveSupport.on_load(:action_controller_base) do
  prepend ContractCsrf
end

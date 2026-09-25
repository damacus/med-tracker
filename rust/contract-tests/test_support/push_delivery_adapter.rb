require 'active_support'
require 'web-push'
require_relative 'push_sender_failure'

module ContractPushDeliveryAdapter
  def payload_send(endpoint:, **)
    case endpoint
    when %r{/contract-push-accepted-}
      true
    when %r{/contract-push-transient-}
      raise SocketError, 'contract provider unavailable'
    when %r{/contract-push-expired-}
      response = Struct.new(:body).new('contract subscription expired')
      raise WebPush::ExpiredSubscription.new(response, 'fcm.googleapis.com')
    else
      raise ArgumentError, 'unexpected contract push endpoint'
    end
  end
end

WebPush.singleton_class.prepend(ContractPushDeliveryAdapter)

ActiveSupport.on_load(:action_controller_base) do
  PushNotificationService.singleton_class.prepend(ContractPushSenderFailure)
end

module ContractPushSenderFailure
  def send_to_account(account, **)
    raise SocketError, 'contract sender unavailable' if account.email.start_with?('contract-push-fatal-')

    super
  end
end

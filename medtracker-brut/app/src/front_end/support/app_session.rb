# Reprensetation of the session your app uses.
# This is intended to provide a rich API
# to whatever is in the sesion, so your app
# is not dealing with a Hash of Whatever.
class AppSession < Brut::FrontEnd::Session

  def signed_guestbook? = !!self.guestbook_message

  
  def guestbook_message
    DB::GuestbookMessage.find(
      external_id: self[:guestbook_message_external_id]
    )
  end

  
  def signed_guestbook(guestbook_message)
    self[:guestbook_message_external_id] = guestbook_message.external_id
  end
end

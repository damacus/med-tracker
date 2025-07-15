class App < Brut::Framework::App
  def id           = "medtracker-brut"
  def organization = "medtracker-brut"

  def initialize
    # Add additional initialization to this initializers.
    # Be sure to use the lazy/block form of store because when this code
    # runs, the app may not have access to networked resources.
    Brut.container.override("session_class",AppSession)
    Brut.container.override("external_id_prefix","mb")
    
    Brut.container.store(
      "trigger_exception_key",
      String,
      "String used to prevent anyone from triggering exceptions in TriggerExceptionHandler"
    ) do
      ENV.fetch("TRIGGER_EXCEPTION_KEY")
    end
  end

  def boot!
    # Perform any additional bootstrapping here, such as configuration of
    # job back-ends or other data stores. When this runs, you can be sure
    # the app has access to networked resources
  end

  # Various class methods may be called here to configure your app.  See the API
  # documentation for Brut::Framework::App.

  # Generic error handler.  Is expected to return a Rack response.
  error do |exception:, http_status_code:|
    SemanticLogger["App"].error(exception || "Unknown error", http_status_code:)
    [ 
      http_status_code || 500,
      {},
      exception&.message || "Internal Server Error '#{http_status_code}'"
    ]
  end

  # Declare your app's routes.
  routes do
    page "/" # Special route for HomePage
    path "/trigger_exception", method: :get
    page "/guestbook"
    form "/guestbook_message"
    page "/new_guestbook_message"
  end
end


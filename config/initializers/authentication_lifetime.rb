Rails.application.config.after_initialize do
  AuthenticationLifetime.inactivity_days
  AuthenticationLifetime.maximum_age_days
  AuthenticationLifetime.api_token_maximum_age_months

  unless ENV['SECRET_KEY_BASE_DUMMY'] || !ApiAppToken.table_exists? || !ApiAppToken.column_names.include?('expires_at')
    ApiAppToken.apply_maximum_age!
  end
end

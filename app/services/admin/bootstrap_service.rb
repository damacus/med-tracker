# frozen_string_literal: true

module Admin
  class BootstrapService
    Result = Struct.new(:success, :error, :user, keyword_init: true) do
      def success?
        success
      end
    end

    REQUIRED_FIELDS = %i[email password name date_of_birth].freeze

    class << self
      def call(email:, password:, name:, date_of_birth:)
        new(email: email, password: password, name: name, date_of_birth: date_of_birth).call
      end
    end

    def initialize(email:, password:, name:, date_of_birth:)
      @email = email.to_s.strip.downcase
      @password = password.to_s
      @name = name.to_s.strip
      @date_of_birth = date_of_birth.to_s
    end

    def call
      validation_error = precondition_error
      return failure(validation_error) if validation_error

      success(create_admin!)
    rescue ArgumentError
      failure('date_of_birth must be a valid date')
    rescue ActiveRecord::RecordInvalid => e
      failure(e.record.errors.full_messages.to_sentence.presence || e.message)
    end

    private

    attr_reader :date_of_birth, :email, :name, :password

    def missing_fields
      REQUIRED_FIELDS.select do |field|
        value = send(field)
        value.blank?
      end
    end

    def email_already_exists?
      Account.exists?(email: email) || User.exists?(email_address: email)
    end

    def precondition_error
      return "Missing required fields: #{missing_fields.join(', ')}" if missing_fields.any?
      return 'An administrator already exists' if administrator_exists?
      return 'Email is already taken' if email_already_exists?

      nil
    end

    def administrator_exists?
      User.administrator.exists?
    end

    def create_admin!
      ActiveRecord::Base.transaction do
        account = create_account!
        person = create_person!(account)
        user = create_user!(person)
        ensure_default_locations!(person)
        user
      end
    end

    def ensure_default_locations!(person)
      location = Location.find_or_create_by!(name: 'Home') do |l|
        l.description = 'Primary home location'
      end
      LocationMembership.find_or_create_by!(location: location, person: person)
    end

    def create_account!
      Account.create!(email: email, password_hash: BCrypt::Password.create(password), status: :verified)
    end

    def create_person!(account)
      Person.create!(
        account: account,
        name: name,
        date_of_birth: parsed_date_of_birth,
        email: email,
        person_type: :adult
      )
    end

    def create_user!(person)
      User.create!(person: person, email_address: email, role: :administrator, active: true)
    end

    def parsed_date_of_birth
      @parsed_date_of_birth ||= Date.parse(date_of_birth)
    end

    def success(user)
      Result.new(success: true, user: user)
    end

    def failure(error)
      Result.new(success: false, error: error)
    end
  end
end

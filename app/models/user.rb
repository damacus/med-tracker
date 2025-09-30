# frozen_string_literal: true

# User model for storing user information and authentication
class User < ApplicationRecord
  belongs_to :person, inverse_of: :user

  accepts_nested_attributes_for :person

  has_secure_password
  has_many :sessions, dependent: :destroy
  has_many :prescriptions, through: :person
  has_many :medicines, through: :prescriptions

  enum :role, { admin: 0, carer: 1, child: 2 }

  normalizes :email_address, with: ->(e) { e.strip.downcase }

  validates :email_address, presence: true,
                            uniqueness: { case_sensitive: false },
                            format: { with: /\A[\w+\-.]+@[a-z\d\-.]+\.[a-z]+\z/i }

  delegate :name, :date_of_birth, :age, to: :person
end

class User < ApplicationRecord
  has_secure_password
  has_many :sessions, dependent: :destroy
  has_many :prescriptions, dependent: :destroy

  enum :role, { admin: 0, carer: 1, child: 2 }

  normalizes :email_address, with: ->(e) { e.strip.downcase }

  validates :name, presence: true
  validates :email_address, presence: true, uniqueness: { case_sensitive: false }, format: { with: /\A[\w+\-.]+@[a-z\d\-.]+\.[a-z]+\z/i }
  validates :date_of_birth, presence: true
end

require 'digest'

class ReplaceInvitationTokenWithDigest < ActiveRecord::Migration[8.1]
  class Invitation < ApplicationRecord
    self.table_name = 'invitations'
  end

  def up
    unless column_exists?(:invitations, :token_digest)
      add_column :invitations, :token_digest, :string
    end

    add_index :invitations, :token_digest, unique: true unless index_exists?(:invitations, :token_digest)

    if column_exists?(:invitations, :token)
      Invitation.reset_column_information
      Invitation.find_each do |invitation|
        next if invitation[:token].blank?

        invitation.update_columns(token_digest: digest(invitation[:token]))
      end
    end

    remove_index :invitations, :token if index_exists?(:invitations, :token)
    remove_column :invitations, :token if column_exists?(:invitations, :token)
  end

  def down
    unless column_exists?(:invitations, :token)
      add_column :invitations, :token, :string
    end
    add_index :invitations, :token, unique: true unless index_exists?(:invitations, :token)

    if column_exists?(:invitations, :token_digest)
      Invitation.reset_column_information
      Invitation.find_each do |invitation|
        next if invitation[:token_digest].blank?

        invitation.update_columns(token: SecureRandom.hex(32))
      end
    end

    remove_index :invitations, :token_digest if index_exists?(:invitations, :token_digest)
    remove_column :invitations, :token_digest if column_exists?(:invitations, :token_digest)
  end

  private

  def digest(token)
    Digest::SHA256.hexdigest(token)
  end
end

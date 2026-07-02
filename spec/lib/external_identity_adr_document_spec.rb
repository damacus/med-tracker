# frozen_string_literal: true

require 'rails_helper'

class ExternalIdentityAdrDocument
  def path = Rails.root.join('docs/adrs/0005-external-identity-provider.md')

  def body = path.read
end

RSpec.describe ExternalIdentityAdrDocument do
  subject(:document) { described_class.new }

  it 'documents the target OIDC provider architecture' do
    expect(document.path).to exist
    expect(document.body).to include('Status: Accepted')
    expect(document.body).to include('External IdP owns primary authentication')
    expect(document.body).to include('web app remains a Rodauth OIDC client')
    expect(document.body).to include('mobile clients use Authorization Code with PKCE')
  end

  it 'chooses the API session exchange strategy and deprecates password login' do
    expect(document.body).to include('exchange external identity for internal API sessions')
    expect(document.body).to include('POST /api/v1/auth/login')
    expect(document.body).to include('deprecated')
    expect(document.body).to include('migration-only')
  end
end

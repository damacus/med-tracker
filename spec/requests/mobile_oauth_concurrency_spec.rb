# frozen_string_literal: true

require 'rails_helper'
require 'timeout'

RSpec.describe 'Concurrent mobile code redemption' do
  self.use_transactional_tests = false

  fixtures :accounts, :people, :users

  let(:verifier) { 'concurrent-mobile-verifier-with-more-than-forty-three-characters' }
  let(:client) do
    OauthApplication.create!(name: 'Concurrent Android', client_id: SecureRandom.uuid, client_kind: :mobile,
                             redirect_uri: 'io.damacus.medtracker:/oauth2redirect',
                             scopes: 'medtracker offline_access', token_endpoint_auth_method: 'none')
  end
  let!(:grant) do
    OauthGrant.create!(account: accounts(:jane_doe), oauth_application: client, client_kind: :mobile,
                       code: SecureRandom.hex(32), code_challenge_method: 'S256',
                       code_challenge: Base64.urlsafe_encode64(Digest::SHA256.digest(verifier), padding: false),
                       redirect_uri: client.redirect_uri, scopes: client.scopes,
                       expires_in: 5.minutes.from_now, authenticated_at: Time.current, last_used_at: Time.current)
  end

  after do
    OauthGrant.where(oauth_application: client).delete_all
    client.destroy!
  end

  it 'issues only one credential when both requests wait on the same grant lock' do
    ready = Queue.new
    workers = []
    params = redemption_params
    grant.with_lock do
      2.times { workers << redemption_worker(params, ready) }
      pids = 2.times.map { Timeout.timeout(10) { ready.pop } }
      expect(pids.uniq.size).to eq(2)
      wait_for_blocked_requests(pids)
    end

    results = workers.map { |worker| Timeout.timeout(10) { worker.value } }
    expect(results.map(&:first).sort).to eq([200, 400])
    expect(OauthGrant.where(oauth_application: client).count).to eq(1)
    expect(grant.reload.code).to be_nil
    token = results.find { |status, _body| status == 200 }.last.fetch('access_token')
    expect(OauthGrant.lookup_by_access_token(token)).to eq(grant)
    get '/api/v1/auth/households', headers: { 'Authorization' => "Bearer #{token}" }, as: :json
    expect(response).to have_http_status(:ok)
  ensure
    workers.each { |worker| worker.kill if worker.alive? }
    workers.each(&:join)
  end

  def redemption_params
    { grant_type: 'authorization_code', client_id: client.client_id,
      redirect_uri: client.redirect_uri, code: grant.code, code_verifier: verifier }
  end

  def redemption_worker(params, ready)
    Thread.new do
      ActiveRecord::Base.connection_pool.with_connection do |connection|
        ready << connection.select_value('SELECT pg_backend_pid()').to_i
        session = ActionDispatch::Integration::Session.new(Rails.application)
        session.post '/token', params: params, as: :json
        [session.response.status, session.response.parsed_body]
      end
    end
  end

  def wait_for_blocked_requests(pids)
    query = "SELECT count(*) FROM pg_stat_activity WHERE pid IN (#{pids.join(',')}) AND wait_event_type = 'Lock'"
    Timeout.timeout(10) do
      loop do
        break if ActiveRecord::Base.connection.select_value(query).to_i == 2

        Thread.pass
      end
    end
  end
end

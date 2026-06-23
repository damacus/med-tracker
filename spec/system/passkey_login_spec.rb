# frozen_string_literal: true

require 'rails_helper'
require 'webauthn/fake_client'

RSpec.describe 'Passkey login', :js do
  fixtures :accounts, :people, :users

  def add_init_script(script)
    page.driver.with_playwright_page do |playwright_page|
      playwright_page.add_init_script(script: script)
    end
  end

  def base_passkey_stub(conditional_available:)
    <<~JS
      const encodeCredential = () => ({
        type: "public-key",
        rawId: Uint8Array.from([1, 2, 3]).buffer,
        response: {
          authenticatorData: Uint8Array.from([4, 5, 6]).buffer,
          clientDataJSON: Uint8Array.from([7, 8, 9]).buffer,
          signature: Uint8Array.from([10, 11, 12]).buffer,
          userHandle: Uint8Array.from([13, 14, 15]).buffer,
        },
      });

      Object.defineProperty(window, "PublicKeyCredential", {
        configurable: true,
        value: class PublicKeyCredential {},
      });
      window.PublicKeyCredential.isConditionalMediationAvailable = async () => #{conditional_available};

      Object.defineProperty(window.navigator, "credentials", {
        configurable: true,
        value: {
          get: async (options) => {
            document.documentElement.dataset.passkeyMediation = options.mediation || "explicit";
            document.documentElement.dataset.passkeyGetCount = String(
              Number(document.documentElement.dataset.passkeyGetCount || "0") + 1,
            );
            return encodeCredential();
          },
        },
      });

      const originalSubmit = HTMLFormElement.prototype.submit;
      HTMLFormElement.prototype.submit = function submit() {
        if (this.id === "webauthn-login-form") {
          document.documentElement.dataset.passkeySubmitCount = String(
            Number(document.documentElement.dataset.passkeySubmitCount || "0") + 1,
          );
          document.documentElement.dataset.passkeyAuthValue =
            this.querySelector("#webauthn-auth")?.value || "";
          return;
        }

        return originalSubmit.call(this);
      };
    JS
  end

  def passkey_browser_stub
    <<~JS
      const decode = (value) => {
        const base64 = value.replace(/-/g, "+").replace(/_/g, "/");
        const padded = base64.padEnd(base64.length + ((4 - (base64.length % 4)) % 4), "=");
        return Uint8Array.from(atob(padded), (char) => char.charCodeAt(0)).buffer;
      };

      Object.defineProperty(window, "PublicKeyCredential", {
        configurable: true,
        value: class PublicKeyCredential {},
      });
      window.PublicKeyCredential.isConditionalMediationAvailable = async () => false;

      Object.defineProperty(window.navigator, "credentials", {
        configurable: true,
        value: {
          get: async () => window.__passkeyCredential,
        },
      });

      window.__setPasskeyCredential = (credential) => {
        window.__passkeyCredential = {
          type: credential.type,
          rawId: decode(credential.rawId),
          response: {
            authenticatorData: decode(credential.response.authenticatorData),
            clientDataJSON: decode(credential.response.clientDataJSON),
            signature: decode(credential.response.signature),
            userHandle: credential.response.userHandle ? decode(credential.response.userHandle) : undefined,
          },
        };
      };
    JS
  end

  def failing_passkey_stub(error_name)
    <<~JS
      Object.defineProperty(window, "PublicKeyCredential", {
        configurable: true,
        value: class PublicKeyCredential {},
      });
      window.PublicKeyCredential.isConditionalMediationAvailable = async () => false;

      Object.defineProperty(window.navigator, "credentials", {
        configurable: true,
        value: {
          get: async () => {
            throw new DOMException("Passkey failed", "#{error_name}");
          },
        },
      });
    JS
  end

  def invalid_passkey_stub
    <<~JS
      Object.defineProperty(window, "PublicKeyCredential", {
        configurable: true,
        value: class PublicKeyCredential {},
      });
      window.PublicKeyCredential.isConditionalMediationAvailable = async () => false;

      Object.defineProperty(window.navigator, "credentials", {
        configurable: true,
        value: {
          get: async () => ({
            type: "public-key",
            rawId: Uint8Array.from([1, 2, 3]).buffer,
            response: {
              authenticatorData: Uint8Array.from([4, 5, 6]).buffer,
              clientDataJSON: Uint8Array.from([7, 8, 9]).buffer,
              signature: Uint8Array.from([10, 11, 12]).buffer,
            },
          }),
        },
      });
    JS
  end

  def current_origin
    uri = URI.parse(current_url)
    "#{uri.scheme}://#{uri.host}:#{uri.port}"
  end

  def configured_webauthn_origin
    ENV.fetch('APP_URL', current_origin)
  end

  def create_passkey_for(account, origin)
    client = WebAuthn::FakeClient.new(origin)
    user_id = account.account_webauthn_user_ids.create!(webauthn_id: WebAuthn.generate_user_id).webauthn_id
    credential = WebAuthn::Credential.from_create(client.create)

    key = account.account_webauthn_keys.create!(
      webauthn_id: credential.id,
      public_key: credential.public_key,
      sign_count: credential.sign_count || 0
    )

    [client, user_id, key]
  end

  def challenge_value
    find('#webauthn-login-form input[name$="challenge"]', visible: :all).value
  end

  def relying_party_for(origin)
    WebAuthn::RelyingParty.new(
      allowed_origins: [origin],
      id: URI.parse(origin).host,
      name: 'MedTracker'
    )
  end

  def prime_passkey_credential(assertion)
    page.execute_script("window.__setPasskeyCredential(#{assertion.to_json})")
  end

  it 'starts conditional passkey autofill on page load when supported' do
    add_init_script(base_passkey_stub(conditional_available: 'true'))

    visit login_path

    expect(page).to have_button('Continue with Passkey')
    expect(page).to have_css('html[data-passkey-mediation="conditional"]', visible: :all)
    expect(page).to have_css('html[data-passkey-submit-count="1"]', visible: :all)
    expect(find_by_id('webauthn-auth', visible: :all).value).to include('"rawId":"AQID"')
  end

  it 'submits the hidden login form after the passkey CTA is clicked' do
    add_init_script(base_passkey_stub(conditional_available: 'false'))

    visit login_path

    expect(page).to have_button('Continue with Passkey')
    click_button 'Continue with Passkey'

    expect(page).to have_css('html[data-passkey-mediation="explicit"]', visible: :all)
    expect(page).to have_css('html[data-passkey-submit-count="1"]', visible: :all)
    expect(find_by_id('webauthn-auth', visible: :all).value).to include('"rawId":"AQID"')
  end

  it 'signs in with a discoverable passkey without entering an email address' do
    add_init_script(passkey_browser_stub)

    visit login_path

    user = users(:carer)
    account = user.person.account
    ensure_api_household_for(user)

    origin = configured_webauthn_origin
    client, user_id, key = create_passkey_for(account, origin)
    assertion = client.get(challenge: challenge_value, user_verified: true, user_handle: user_id)
    assertion_credential = WebAuthn::Credential.from_get(assertion, relying_party: relying_party_for(origin))

    expect(
      assertion_credential.verify(challenge_value, public_key: key.public_key, sign_count: key.sign_count)
    ).to be(true)

    prime_passkey_credential(assertion)

    click_button 'Continue with Passkey'

    expect(page).to have_current_path(expected_dashboard_path_for(account.email))
  end

  it 'shows a helpful message when explicit passkey sign-in is cancelled' do
    add_init_script(failing_passkey_stub('NotAllowedError'))

    visit login_path
    click_button 'Continue with Passkey'

    expect(page).to have_css(
      '#passkey-login-error',
      text: 'Passkey sign-in was cancelled. Try again or use your password.'
    )
    expect(page).to have_field('email')
    expect(page).to have_field('password')
  end

  it 'shows a friendly failure when the selected passkey is unknown' do
    add_init_script(invalid_passkey_stub)

    visit login_path
    click_button 'Continue with Passkey'

    expect(page).to have_current_path('/login')
    expect(page).to have_text('We could not sign you in with that passkey. Try again or use your password.')
  end

  it 'keeps the passkey CTA hidden when WebAuthn is unavailable' do
    add_init_script(<<~JS)
      delete globalThis.PublicKeyCredential;
      Object.defineProperty(Navigator.prototype, "credentials", {
        configurable: true,
        get() {
          return undefined;
        },
      });
    JS

    visit login_path

    expect(page).to have_field('email')
    expect(page).to have_field('password')
    expect(page).to have_no_button('Continue with Passkey')
    expect(page).to have_css('#passkey-login-section[hidden]', visible: :all)
  end
end

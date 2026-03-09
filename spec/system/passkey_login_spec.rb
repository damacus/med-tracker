# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Passkey login', :js do
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

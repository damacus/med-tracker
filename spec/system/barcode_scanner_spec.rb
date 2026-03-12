# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'BarcodeScanner' do
  fixtures :accounts, :people, :users

  let(:user) { users(:john) }

  def stub_html5_qrcode(cameras:, error: nil, camera_delay_ms: 0)
    page.execute_script(<<~JS)
      (() => {
        const cameras = #{cameras.to_json};
        const error = #{error.to_json};
        const cameraDelayMs = #{camera_delay_ms};

        const buildError = (details) => {
          if (!details) return null;

          const instance = new Error(details.message);
          instance.name = details.name;
          return instance;
        };

        class FakeHtml5Qrcode {
          static getCameras() {
            window.__barcodeScannerGetCamerasCalls = (window.__barcodeScannerGetCamerasCalls || 0) + 1;

            const failure = error && error.phase === "getCameras" ? buildError(error) : null;
            if (failure) return Promise.reject(failure);

            return new Promise((resolve) => {
              window.setTimeout(() => resolve(cameras), cameraDelayMs);
            });
          }

          constructor(elementId) {
            this.elementId = elementId;
            this.state = 1;
          }

          start(cameraId, config) {
            window.__barcodeScannerLastStartedCameraId = cameraId;
            window.__barcodeScannerLastStartConfig = config;

            const failure = error && error.phase === "start" ? buildError(error) : null;
            if (failure) return Promise.reject(failure);

            this.state = 2;
            return Promise.resolve();
          }

          getState() {
            return this.state;
          }

          stop() {
            this.state = 1;
            return Promise.resolve();
          }

          clear() {
            return Promise.resolve();
          }
        }

        window.__barcodeScannerTestLibrary = { Html5Qrcode: FakeHtml5Qrcode };
      })();
    JS
  end

  context 'with static HTML (rack_test)' do
    before do
      driven_by(:rack_test)
      login_as(user)
    end

    it 'displays the barcode scanner on the medication finder page' do
      visit medication_finder_path

      within '[data-testid="barcode-scanner"]' do
        aggregate_failures 'scanner component elements' do
          expect(page).to have_button('Start Scanner')
          expect(page).to have_button('Stop Scanner', visible: :hidden)
          expect(page).to have_content('Or enter barcode manually')
          expect(page).to have_field('manual-barcode')
          expect(page).to have_button('Submit')
        end
      end
    end

    it 'renders the scanner region as hidden initially' do
      visit medication_finder_path

      scanner_region = find_by_id('barcode-scanner-region', visible: false)
      expect(scanner_region).not_to be_visible
    end

    it 'provides manual barcode input as a fallback' do
      visit medication_finder_path

      within '[data-testid="manual-barcode-input"]' do
        expect(page).to have_field('manual-barcode', type: 'text')
        expect(page).to have_button('Submit')
      end
    end
  end

  context 'with JavaScript (Playwright)' do
    before do
      driven_by(:playwright)
      login_as(user)
    end

    it 'requests camera access and starts scanning with the preferred available camera' do
      visit medication_finder_path
      stub_html5_qrcode(
        cameras: [
          { id: 'front-camera', label: 'FaceTime HD Camera' },
          { id: 'rear-camera', label: 'Rear Ultra Wide Camera' }
        ]
      )

      scanner = find('[data-testid="barcode-scanner"]')
      expect(scanner['data-scanner-state']).to eq('idle')

      click_button 'Start Scanner'

      using_wait_time(10) do
        expect(page).to have_text('Point your camera at a barcode')
        expect(find_by_id('barcode-scanner-region')).to be_visible
      end

      expect(find('[data-testid="barcode-scanner"]')['data-scanner-state']).to eq('scanning')
      expect(page.evaluate_script('window.__barcodeScannerGetCamerasCalls')).to eq(1)
      expect(page.evaluate_script('window.__barcodeScannerLastStartedCameraId')).to eq('rear-camera')
    end

    it 'keeps the scanner region visible while camera access is being requested' do
      visit medication_finder_path
      stub_html5_qrcode(
        cameras: [{ id: 'front-camera', label: 'FaceTime HD Camera' }],
        camera_delay_ms: 250
      )

      click_button 'Start Scanner'

      expect(page).to have_text('Requesting camera access')
      expect(find_by_id('barcode-scanner-region')).to be_visible
    end

    it 'shows a denied state when camera permission is rejected' do
      visit medication_finder_path
      stub_html5_qrcode(
        cameras: [],
        error: { phase: 'getCameras', name: 'NotAllowedError', message: 'Permission denied' }
      )

      click_button 'Start Scanner'

      using_wait_time(10) do
        expect(page).to have_text('Camera access was denied. Please use manual entry below.')
        expect(page).to have_button('Start Scanner')
      end

      expect(find('[data-testid="barcode-scanner"]')['data-scanner-state']).to eq('denied')
    end

    it 'shows an unavailable state when no cameras are returned' do
      visit medication_finder_path
      stub_html5_qrcode(cameras: [])

      click_button 'Start Scanner'

      using_wait_time(10) do
        expect(page).to have_text('No camera was found. Please use manual entry below.')
        expect(page).to have_button('Start Scanner')
      end

      expect(find('[data-testid="barcode-scanner"]')['data-scanner-state']).to eq('unavailable')
    end

    it 'returns to an idle, restartable state after stopping the scanner' do
      visit medication_finder_path
      stub_html5_qrcode(cameras: [{ id: 'front-camera', label: 'FaceTime HD Camera' }])

      click_button 'Start Scanner'

      using_wait_time(10) do
        expect(page).to have_button('Stop Scanner')
      end

      click_button 'Stop Scanner'

      using_wait_time(10) do
        expect(page).to have_button('Start Scanner')
      end

      expect(find('[data-testid="barcode-scanner"]')['data-scanner-state']).to eq('idle')

      click_button 'Start Scanner'

      using_wait_time(10) do
        expect(page).to have_button('Stop Scanner')
      end

      expect(page.evaluate_script('window.__barcodeScannerGetCamerasCalls')).to eq(2)
      expect(page).to have_no_button('Start Scanner')
    end
  end
end

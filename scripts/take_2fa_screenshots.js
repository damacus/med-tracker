// Script to take screenshots of the 2FA management interface
const { chromium } = require('playwright');

(async () => {
  const browser = await chromium.launch();
  const context = await browser.newContext({
    viewport: { width: 1280, height: 1024 }
  });
  const page = await context.newPage();

  try {
    // Navigate to login page
    await page.goto('http://localhost:3000/login');
    
    // Login as damacus user
    await page.fill('input[name="email"]', 'damacus@example.com');
    await page.fill('input[name="password"]', 'password');
    await page.click('button[type="submit"]');
    
    // Wait for navigation to dashboard
    await page.waitForURL('**/dashboard', { timeout: 10000 });
    
    // Navigate to profile page
    await page.goto('http://localhost:3000/profile');
    await page.waitForLoadState('networkidle');
    
    // Take full page screenshot
    await page.screenshot({ 
      path: 'docs/screenshots/profile-2fa-full.png',
      fullPage: true 
    });
    
    console.log('Screenshot saved: docs/screenshots/profile-2fa-full.png');
    
    // Take screenshot of just the 2FA card
    const twoFactorCard = await page.locator('text=Two-Factor Authentication').locator('..').locator('..');
    if (await twoFactorCard.isVisible()) {
      await twoFactorCard.screenshot({ 
        path: 'docs/screenshots/profile-2fa-card.png' 
      });
      console.log('Screenshot saved: docs/screenshots/profile-2fa-card.png');
    }
    
  } catch (error) {
    console.error('Error taking screenshots:', error);
    process.exit(1);
  } finally {
    await browser.close();
  }
})();

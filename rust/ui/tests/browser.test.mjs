import assert from 'node:assert/strict';
import {after, before, test} from 'node:test';
import {createServer} from 'node:http';
import {readFile, mkdir} from 'node:fs/promises';
import {chromium} from 'playwright';

let browser, server, base;
const hydrated = process.env.UI_MODE === 'hydrate';
const hydrateBoot = `import init, {hydrate} from '/pkg/portable_ui_hydrated_consumer.js';
window.serverSaveButton = document.getElementById('save-button');
await init();
hydrate();
if (window.serverSaveButton !== document.getElementById('save-button')) throw new Error('SSR DOM was replaced');
`;
const boot = `document.getElementById('preferences-form').addEventListener('submit', event => {
  event.preventDefault();
  const count = document.getElementById('save-count');
  count.textContent = String(Number(count.textContent) + 1);
});
window.uiController = window.DamacusUI?.init(document);
document.documentElement.dataset.uiReady = 'true';`;

before(async () => {
  server = createServer(async (request, response) => {
    try {
      const url = new URL(request.url, 'http://localhost');
      if (url.pathname.startsWith('/pkg/') && /^\/pkg\/[a-z_]+\.(js|wasm)$/.test(url.pathname)) {
        response.setHeader('Content-Type',url.pathname.endsWith('.wasm') ? 'application/wasm' : 'text/javascript');
        response.end(await readFile(new URL('../.test-output'+url.pathname,import.meta.url)));
      } else if (url.pathname === '/ui.js') {
        response.setHeader('Content-Type','text/javascript');
        response.end(await readFile(new URL('../src/runtime.js',import.meta.url)));
      } else if (url.pathname === '/start.js') {
        response.setHeader('Content-Type','text/javascript');
        response.end((hydrated ? hydrateBoot : "")+boot);
      } else {
        response.setHeader('Content-Type','text/html');
        const name = ['dialog','sheet'].includes(url.searchParams.get('open')) ? url.searchParams.get('open') : 'closed';
        let html = await readFile(new URL(`../.test-output/${name}.html`,import.meta.url),'utf8');
        if (hydrated) html = html.replace('<script defer src="/start.js">','<script type="module" src="/start.js">');
        response.end(html);
      }
    } catch (error) { response.statusCode=500;response.end(String(error)); }
  });
  await new Promise(resolve => server.listen(0,'127.0.0.1',resolve));
  base = `http://127.0.0.1:${server.address().port}`;
  browser = await chromium.launch({headless:true});
});
after(async () => { await browser?.close(); await new Promise(resolve => server?.close(resolve)); });

async function pageFor(query='') {
  const page = await browser.newPage({viewport:{width:390,height:844}});
  page.setDefaultTimeout(3000);
  const errors = [];
  page.on('pageerror',error => errors.push(error.message));
  page.on('console',message => { if (message.type()==='error') errors.push(message.text()); });
  await page.goto(base+query);
  await page.locator('html[data-ui-ready=true]').waitFor();
  if (hydrated) assert.equal(await page.locator('html').getAttribute('data-hydrated'),'true');
  assert.deepEqual(errors,[]);
  return page;
}

test('native Button and Field work in an unbranded consumer',async () => {
  const page = await pageFor();
  try {
    await page.getByLabel('Display name').fill('New name');
    assert.equal(await page.locator('#account-name').getAttribute('aria-describedby'),'account-name-hint account-name-error');
    assert.equal(await page.locator('#disabled-name').isDisabled(),true);
    assert.equal(await page.locator('#disabled-button').isDisabled(),true);
    await page.locator('#save-button').focus();
    await page.keyboard.press('Enter');
    await page.keyboard.press('Space');
    assert.equal(await page.locator('#save-count').textContent(),'2');
  } finally { await page.close(); }
});

test('Tabs skip disabled choices, activate panels and keep one tab stop',async () => {
  const page = await pageFor();
  try {
    await page.locator('#general-tab').focus();
    await page.keyboard.press('ArrowRight');
    assert.equal(await page.locator('#security-tab').getAttribute('aria-selected'),'true');
    assert.equal(await page.locator('#security-panel').isVisible(),true);
    assert.equal(await page.locator('#general-panel').isVisible(),false);
    assert.equal(await page.locator('[role=tab][tabindex="0"]').count(),1);
    await page.keyboard.press('Home');
    assert.equal(await page.locator('#general-tab').getAttribute('aria-selected'),'true');
    await page.keyboard.press('End');
    assert.equal(await page.locator('#security-tab').getAttribute('aria-selected'),'true');
  } finally { await page.close(); }
});

for (const [id,opener,close] of [['confirmation','open-dialog','Close confirmation'],['appearance','open-sheet','Close appearance']]) {
  test(`${id}: native modality blocks background focus and restores opener`,async () => {
    const page = await pageFor();
    try {
      await page.locator(`#${opener}`).click();
      await page.locator(`#${id}:modal`).waitFor();
      await page.evaluate(() => document.getElementById('save-button').focus());
      assert.equal(await page.evaluate(id => document.getElementById(id).contains(document.activeElement),id),true);
      for (const key of ['Tab','Shift+Tab','Tab']) {
        await page.keyboard.press(key);
        assert.equal(await page.evaluate(() => document.activeElement.id === "save-button"),false);
      }
      assert.equal(await page.evaluate(() => document.documentElement.style.overflow),'hidden');
      const client = await page.context().newCDPSession(page);
      const snapshot = await client.send('Accessibility.getFullAXTree');
      assert.equal(snapshot.nodes.some(node => !node.ignored && node.role?.value === 'button' && node.name?.value === 'Save'),false);
      await client.detach();
      await page.keyboard.press('Escape');
      await page.locator(`#${id}:modal`).waitFor({state:'hidden'});
      assert.equal(await page.evaluate(opener => document.activeElement.id===opener,opener),true);
      await page.waitForFunction(() => document.documentElement.style.overflow !== 'hidden');
      await page.locator(`#${opener}`).click();
      await page.getByRole('button',{name:close,exact:true}).click();
      assert.equal(await page.locator(`#${id}:modal`).count(),0);
    } finally { await page.close(); }
  });
}

test('initial server-open overlays become real native modals',async () => {
  for (const [open,id] of [['dialog','confirmation'],['sheet','appearance']]) {
    const response = await fetch(base+'?open='+open);
    assert.match(await response.text(),new RegExp(`<dialog[^>]*id="${id}"[^>]*open`));
    const page = await pageFor('?open='+open);
    try {
      assert.equal(await page.locator(`#${id}:modal`).count(),1);
      await page.keyboard.press('Escape');
      assert.equal(await page.locator(`#${id}:modal`).count(),0);
    } finally { await page.close(); }
  }
});

test('explicit lifecycle is idempotent and can be disposed',async () => {
  const page = await pageFor();
  try {
    assert.equal(await page.evaluate(() => window.DamacusUI.init(document)===window.uiController),true);
    await page.evaluate(() => window.uiController.dispose());
    await page.locator('#open-dialog').click();
    assert.equal(await page.locator('#confirmation:modal').count(),0);
    await page.evaluate(() => window.DamacusUI.init(document));
    await page.locator('#open-dialog').click();
    assert.equal(await page.locator('#confirmation:modal').count(),1);
    await mkdir(new URL('../../../docs/screenshots/',import.meta.url),{recursive:true});
    await page.screenshot({path:new URL('../../../docs/screenshots/portable-ui-neutral-mobile.png',import.meta.url).pathname});
    await page.setViewportSize({width:1280,height:900});
    await page.screenshot({path:new URL('../../../docs/screenshots/portable-ui-neutral-desktop.png',import.meta.url).pathname});
  } finally { await page.close(); }
});

test('hydration preserves SSR nodes and wires Leptos application events', {skip:!hydrated}, async () => {
  const page = await pageFor();
  try {
    assert.equal(await page.evaluate(() => window.serverSaveButton===document.getElementById('save-button')),true);
    await page.locator('#reactive-button').click();
    await page.waitForFunction(() => document.getElementById('reactive-button').textContent.includes('1'));
  } finally { await page.close(); }
});

test('backdrop dismissal restores existing scroll styles',async () => {
  const page = await pageFor();
  try {
    await page.evaluate(() => { document.documentElement.style.overflow='scroll'; document.body.style.overflow='auto'; });
    await page.locator('#open-dialog').click();
    await page.mouse.click(1,1);
    await page.waitForFunction(() => document.documentElement.style.overflow==='scroll');
    assert.equal(await page.evaluate(() => document.body.style.overflow),'auto');
    assert.equal(await page.locator('#confirmation:modal').count(),0);
    await page.evaluate(() => window.uiController.dispose());
    assert.equal(await page.evaluate(() => document.documentElement.style.overflow),'scroll');
  } finally { await page.close(); }
});

test('stacked native overlays release scrolling only after the last close',async () => {
  const page = await pageFor();
  try {
    await page.locator('#open-dialog').click();
    await page.evaluate(() => {
      const button=document.createElement('button');
      button.textContent='Nested appearance';
      button.setAttribute('data-ui-open','appearance');
      document.getElementById('confirmation').append(button);
    });
    await page.getByRole('button',{name:'Nested appearance'}).click();
    assert.equal(await page.locator('dialog:modal').count(),2);
    await page.keyboard.press('Escape');
    await page.waitForFunction(() => !document.getElementById('appearance').open);
    assert.equal(await page.evaluate(() => document.documentElement.style.overflow),'hidden');
    await page.keyboard.press('Escape');
    await page.waitForFunction(() => document.documentElement.style.overflow!=='hidden');
    assert.equal(await page.locator('dialog:modal').count(),0);
  } finally { await page.close(); }
});

test('application adapters can retain existing markup and navigation routes',async () => {
  const page = await pageFor();
  try {
    await page.evaluate(() => {
      window.uiController.dispose();
      document.getElementById('open-dialog').setAttribute('data-product-open','confirmation');
      document.getElementById('confirmation').classList.add('product-dialog');
      const tabs=document.createElement('nav');
      tabs.id='product-tabs';
      tabs.className='product-tabs';
      tabs.innerHTML='<a id="first" role="tab" aria-selected="true" href="?section=first">First</a><a id="second" role="tab" aria-selected="false" href="?section=second">Second</a>';
      document.body.prepend(tabs);
      window.uiController=window.DamacusUI.init(document, {modalSelector:'.product-dialog',triggerAttribute:'data-product-open',tabsSelector:'.product-tabs',navigation:true,focusStorageKey:'product-focus'});
    });
    await page.locator('#open-dialog').click();
    assert.equal(await page.locator('#confirmation:modal').count(),1);
    await page.keyboard.press('Escape');
    await page.locator('#first').focus();
    await page.keyboard.press('ArrowRight');
    await page.waitForURL('**/?section=second');
    assert.equal(await page.evaluate(() => sessionStorage.getItem('product-focus')),'second');
  } finally { await page.close(); }
});

test('navigation focus is restored once and stale destinations are discarded',async () => {
  const page = await pageFor();
  try {
    await page.evaluate(() => {
      window.uiController.dispose();
      sessionStorage.setItem('damacus-ui-tab-focus','security-tab');
      document.getElementById('security-tab').setAttribute('aria-selected','true');
      window.uiController=window.DamacusUI.init(document);
    });
    assert.equal(await page.evaluate(() => document.activeElement.id),'security-tab');
    assert.equal(await page.evaluate(() => sessionStorage.getItem('damacus-ui-tab-focus')),null);
    await page.evaluate(() => {
      window.uiController.dispose();
      sessionStorage.setItem('damacus-ui-tab-focus','missing-tab');
      window.uiController=window.DamacusUI.init(document);
    });
    assert.equal(await page.evaluate(() => sessionStorage.getItem('damacus-ui-tab-focus')),null);
  } finally { await page.close(); }
});

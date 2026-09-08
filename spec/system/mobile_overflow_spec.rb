# frozen_string_literal: true

require 'rails_helper'

module MobileOverflowCardGeometry
  CARD_ACTION_GEOMETRY_SCRIPT = <<~JAVASCRIPT
    (() => {
      const cards = __CARDS__;
      const viewportWidth = document.documentElement.clientWidth;
      const within = (inner, outer, allowance = 1) =>
        inner.left >= outer.left - allowance && inner.right <= outer.right + allowance;
      const textRects = (element) => {
        const walker = document.createTreeWalker(element, NodeFilter.SHOW_TEXT);
        const rects = [];
        let node = walker.nextNode();

        while (node) {
          const range = document.createRange();
          range.selectNodeContents(node);
          rects.push(...Array.from(range.getClientRects()).filter((rect) => rect.width > 0 && rect.height > 0));
          node = walker.nextNode();
        }

        return rects;
      };

      return cards.map(({ selector, action_testid: actionTestid }) => {
        const card = document.querySelector(selector);
        const actions = card?.querySelector(`[data-testid="${actionTestid}"]`);
        const controls = actions
          ? Array.from(actions.querySelectorAll('a, button')).filter((element) => {
              const rect = element.getBoundingClientRect();
              return rect.width > 0 && rect.height > 0;
            })
          : [];
        const cardRect = card?.getBoundingClientRect();
        const actionsRect = actions?.getBoundingClientRect();
        const controlsWithLabels = controls.map((element) => {
          const controlRect = element.getBoundingClientRect();
          const labels = textRects(element);

          return {
            controlRect,
            labels,
            labelsWithinControl: labels.length > 0 && labels.every((label) => within(label, controlRect)),
            labelsWithinCard: cardRect && labels.length > 0 &&
              labels.every((label) => within(label, cardRect))
          };
        });

        return {
          selector,
          cardWithinViewport: Boolean(cardRect && cardRect.left >= -1 && cardRect.right <= viewportWidth + 1),
          actionWithinCard: Boolean(cardRect && actionsRect && within(actionsRect, cardRect)),
          actionContentContained: Boolean(actions && actions.scrollWidth <= actions.clientWidth + 1),
          controlsWithinCard: Boolean(cardRect && controls.every((element) =>
            within(element.getBoundingClientRect(), cardRect)
          )),
          controlContentContained: Boolean(controls.length > 0 && controls.every((element) =>
            element.scrollWidth <= element.clientWidth + 1
          )),
          controlLabelWithinControl: Boolean(controlsWithLabels.length > 0 && controlsWithLabels.every((control) =>
            control.labelsWithinControl
          )),
          controlLabelWithinCard: Boolean(controlsWithLabels.length > 0 && controlsWithLabels.every((control) =>
            control.labelsWithinCard
          )),
          pageWithinViewport: Math.max(document.documentElement.scrollWidth, document.body.scrollWidth) -
            document.documentElement.clientWidth <= 1
        };
      });
    })()
  JAVASCRIPT
end

module MobileOverflowFocusGeometry
  FOCUS_GEOMETRY_SCRIPT = <<~JAVASCRIPT
    (() => {
      const element = document.querySelector(__SELECTOR__);
      const card = document.querySelector(__CARD_SELECTOR__);
      element?.focus({ preventScroll: true });
      const elementRect = element?.getBoundingClientRect();
      const cardRect = card?.getBoundingClientRect();
      const styles = element ? window.getComputedStyle(element) : null;

      return {
        focused: Boolean(element && document.activeElement === element),
        focusVisible: Boolean(element && element.matches(':focus-visible')),
        withinCard: Boolean(elementRect && cardRect &&
          elementRect.left >= cardRect.left - 1 && elementRect.right <= cardRect.right + 1 &&
          elementRect.top >= cardRect.top - 1 && elementRect.bottom <= cardRect.bottom + 1),
        visible: Boolean(elementRect && styles && styles.display !== 'none' &&
          styles.visibility !== 'hidden' && elementRect.width > 0 && elementRect.height > 0)
      };
    })()
  JAVASCRIPT
end

module MobileOverflowActionMenuGeometry
  ACTION_MENU_GEOMETRY_SCRIPT = <<~JAVASCRIPT
    (() => {
      const menu = document.querySelector('[data-testid="__MENU_TESTID__"]');
      const viewportWidth = document.documentElement.clientWidth;
      const viewportHeight = document.documentElement.clientHeight;
      const items = menu
        ? Array.from(menu.querySelectorAll('[role="menuitem"]')).filter((element) => {
            const rect = element.getBoundingClientRect();
            return rect.width > 0 && rect.height > 0;
          })
        : [];
      const menuRect = menu?.getBoundingClientRect();

      return {
        menuWithinViewport: Boolean(menuRect && menuRect.left >= -1 && menuRect.right <= viewportWidth + 1 &&
          menuRect.top >= -1 && menuRect.bottom <= viewportHeight + 1),
        itemsWithinViewport: Boolean(items.length > 0 && items.every((element) => {
          const rect = element.getBoundingClientRect();
          return rect.left >= -1 && rect.right <= viewportWidth + 1 &&
            rect.top >= -1 && rect.bottom <= viewportHeight + 1;
        })),
        itemContentContained: Boolean(items.length > 0 && items.every((element) =>
          element.scrollWidth <= element.clientWidth + 1
        ))
      };
    })()
  JAVASCRIPT
end

module MobileOverflowLongContentGeometry
  SHORTCUT_GEOMETRY_SCRIPT = <<~JAVASCRIPT
    (() => Array.from(document.querySelectorAll('[data-testid="mobile-rail"] a')).map((link) => {
      const linkRect = link.getBoundingClientRect();
      const label = link.querySelector('span');
      const labelRect = label?.getBoundingClientRect();

      return {
        label: label?.textContent.trim(),
        linkRect: { left: linkRect.left, right: linkRect.right, top: linkRect.top, bottom: linkRect.bottom },
        labelRect: labelRect && {
          left: labelRect.left, right: labelRect.right, top: labelRect.top, bottom: labelRect.bottom
        },
        usableHeight: linkRect.height >= 44,
        labelWithinLink: Boolean(labelRect && labelRect.left >= linkRect.left - 1 &&
          labelRect.right <= linkRect.right + 1 && labelRect.top >= linkRect.top - 1 &&
          labelRect.bottom <= linkRect.bottom + 1),
        labelContained: Boolean(label && label.scrollWidth <= label.clientWidth + 1 &&
          label.scrollHeight <= label.clientHeight + 1)
      };
    }))()
  JAVASCRIPT

  INVITATION_GEOMETRY_SCRIPT = <<~JAVASCRIPT
    (() => {
      const email = Array.from(document.querySelectorAll('#admin_invitations p'))
        .find((element) => element.textContent.trim() === __EMAIL__);
      const row = email?.closest('[class*="md:flex-row"]');
      const emailRect = email?.getBoundingClientRect();
      const rowRect = row?.getBoundingClientRect();
      const actions = row ? Array.from(row.querySelectorAll('button, a')).filter((element) => {
        const rect = element.getBoundingClientRect();
        return rect.width > 0 && rect.height > 0;
      }) : [];

      return {
        found: Boolean(email && row),
        emailWithinRow: Boolean(emailRect && rowRect && emailRect.left >= rowRect.left - 1 &&
          emailRect.right <= rowRect.right + 1),
        emailContained: Boolean(email && email.scrollWidth <= email.clientWidth + 1),
        actionsReachable: Boolean(actions.length > 0 && actions.every((element) => {
          const rect = element.getBoundingClientRect();
          return rect.left >= -1 && rect.right <= document.documentElement.clientWidth + 1 &&
            rect.top >= -1 && rect.bottom <= document.documentElement.scrollHeight;
        })),
        emailRect: emailRect && { left: emailRect.left, right: emailRect.right, width: emailRect.width },
        rowRect: rowRect && { left: rowRect.left, right: rowRect.right, width: rowRect.width },
        pageOverflow: Math.max(document.documentElement.scrollWidth, document.body.scrollWidth) -
          document.documentElement.clientWidth
      };
    })()
  JAVASCRIPT
end

RSpec.describe 'Mobile overflow handling' do
  fixtures :all

  before do
    admin = users(:admin)
    login_as(admin)
    create_household_audit_version(admin)
  end

  after do
    PaperTrail.request.controller_info = {}
    PaperTrail.request.whodunnit = nil
  end

  it 'keeps medication header actions inside the mobile viewport', :js do
    page.current_window.resize_to(390, 844)

    visit medications_path

    expect(page_horizontal_overflow).to be <= 1
    expect(offscreen_header_actions).to be_empty
  end

  it 'keeps removed quick action chrome from causing mobile overflow', :js do
    page.current_window.resize_to(390, 844)

    visit dashboard_path

    expect(page).to have_css('[data-testid="mobile-rail"]')
    expect(page).to have_no_css('[data-testid="floating-action-menu-toggle"]')
    expect(page).to have_no_css('[data-testid="floating-action-menu-items"]')
    expect(page_horizontal_overflow).to be <= 1
  end

  it 'contains long translated mobile shortcut labels at phone widths', :js do
    users(:admin).person.account.update!(mobile_shortcuts: %w[finder medicine_reviews administration])

    %i[en pt cy].each do |locale|
      allow(I18n).to receive(:locale).and_return(locale)

      [320, 390].each do |width|
        page.current_window.resize_to(width, 844)
        visit dashboard_path

        geometry = page.evaluate_script(MobileOverflowLongContentGeometry::SHORTCUT_GEOMETRY_SCRIPT)

        expect(geometry.size).to eq(3), "locale=#{locale} width=#{width} geometry=#{geometry.inspect}"
        expect(geometry).to all(
          include(
            'usableHeight' => true,
            'labelWithinLink' => true,
            'labelContained' => true
          )
        ), "locale=#{locale} width=#{width} geometry=#{geometry.inspect}"
        expect(page_horizontal_overflow).to be <= 1, "locale=#{locale} width=#{width}"
      end
    end
  end

  it 'contains a long invitation email while keeping row actions reachable', :aggregate_failures, :js do
    local_part = "long#{'x' * 60}"
    email = "#{local_part}@example.com"
    HouseholdInvitation.create!(
      household: browser_household,
      invited_by_membership: browser_membership,
      email: email,
      membership_role: :member
    )

    [320, 390].each do |width|
      page.current_window.resize_to(width, 844)
      visit admin_invitations_path

      geometry = page.evaluate_script(
        MobileOverflowLongContentGeometry::INVITATION_GEOMETRY_SCRIPT.sub('__EMAIL__', email.to_json)
      )

      expect(geometry).to include(
        'found' => true,
        'emailWithinRow' => true,
        'emailContained' => true,
        'actionsReachable' => true
      ), "width=#{width} geometry=#{geometry.inspect}"
      expect(geometry.fetch('pageOverflow')).to be <= 1, "width=#{width} geometry=#{geometry.inspect}"
    end
  end

  it 'uses mobile cards without page-level overflow on dense table pages', :js do
    page.current_window.resize_to(390, 844)

    {
      schedules_path => 'schedules-mobile-list',
      admin_users_path => 'admin-users-mobile-list',
      admin_carer_relationships_path => 'admin-carer-relationships-mobile-list',
      admin_audit_logs_path => 'admin-audit-logs-mobile-list'
    }.each do |path, testid|
      visit path

      expect(page).to have_css(%([data-testid="#{testid}"]))
      expect(page).to have_no_table
      expect(page_horizontal_overflow).to be <= 1
    end
  end

  it 'wraps long audit event names inside the mobile card header', :js do
    long_event = 'auth_token/native_device_token/credential_rotation_completed'
    version = create_household_audit_version(users(:admin), event: long_event)
    page.current_window.resize_to(390, 844)

    visit admin_audit_logs_path

    geometry = page.evaluate_script(<<~JS)
      (() => {
        const card = document.querySelector(
          '[data-testid="admin-audit-logs-mobile-list"] [data-version-id="#{version.id}"]'
        );
        const badge = Array.from(card.querySelectorAll('span')).find((element) =>
          element.textContent.includes('Credential Rotation Completed')
        );
        const cardRect = card.getBoundingClientRect();
        const badgeRect = badge.getBoundingClientRect();
        return {
          cardLeft: cardRect.left,
          cardRight: cardRect.right,
          cardMidpoint: cardRect.left + (cardRect.width / 2),
          badgeLeft: badgeRect.left,
          badgeRight: badgeRect.right,
          overflow: document.documentElement.scrollWidth - document.documentElement.clientWidth
        };
      })()
    JS

    expect(geometry.fetch('overflow')).to be <= 1
    expect(geometry.fetch('badgeLeft')).to be >= geometry.fetch('cardMidpoint')
    expect(geometry.fetch('badgeRight')).to be <= geometry.fetch('cardRight')
  end

  it 'keeps desktop tables visible on dense table pages', :js do
    page.current_window.resize_to(1024, 900)

    {
      schedules_path => 'schedules-desktop-table',
      admin_users_path => 'admin-users-desktop-table',
      admin_carer_relationships_path => 'admin-carer-relationships-desktop-table',
      admin_audit_logs_path => 'admin-audit-logs-desktop-table'
    }.each do |path, testid|
      visit path

      expect(page).to have_css(%([data-testid="#{testid}"] table), visible: :visible)
    end
  end

  it 'keeps schedule and person-medication card actions contained across themes and widths', :js do
    medications(:paracetamol).update!(
      name: 'Paracetamol extended release oral suspension with an intentionally long label'
    )
    medications(:vitamin_d).update!(
      name: 'Vitamin D high-strength daily supplement with an intentionally long label'
    )

    schedule = schedules(:john_paracetamol)
    person_medication = person_medications(:john_vitamin_d)
    cards = [
      {
        kind: 'schedule',
        selector: "##{tenant_dom_id(schedule)}",
        action_testid: 'schedule-card-actions',
        past_testid: "log-past-dose-schedule-#{schedule.id}",
        trigger_testid: "schedule-actions-#{schedule.id}",
        menu_testid: "schedule-actions-menu-#{schedule.id}"
      },
      {
        kind: 'person-medication',
        selector: "##{tenant_dom_id(person_medication)}",
        action_testid: 'person-medication-card-actions',
        past_testid: "log-past-dose-person-medication-#{person_medication.id}",
        trigger_testid: "person-medication-actions-#{person_medication.id}",
        menu_testid: "person-medication-actions-menu-#{person_medication.id}"
      }
    ]

    [320, 390, 768, 1280].each do |width|
      page.current_window.resize_to(width, width == 1280 ? 900 : 844)

      %w[light dark].each do |appearance|
        apply_appearance(appearance)
        visit person_path(people(:john))

        expect(page).to have_css("#{cards.first[:selector]} [data-testid='#{cards.first[:action_testid]}']")
        expect(page).to have_css("#{cards.last[:selector]} [data-testid='#{cards.last[:action_testid]}']")

        geometry = card_action_geometry(cards)
        expect(geometry).to all(
          include(
            'cardWithinViewport' => true,
            'actionWithinCard' => true,
            'actionContentContained' => true,
            'controlsWithinCard' => true,
            'controlContentContained' => true,
            'controlLabelWithinControl' => true,
            'controlLabelWithinCard' => true,
            'pageWithinViewport' => true
          )
        ), "card geometry at #{width}px in #{appearance}: #{geometry.inspect}"
        save_page_screenshot(width: width, appearance: appearance)

        cards.each do |card|
          [card[:past_testid], card[:trigger_testid]].each do |testid|
            focus = focus_geometry("#{card[:selector]} [data-testid='#{testid}']", card[:selector])
            expect(focus).to include(
              'focused' => true,
              'focusVisible' => true,
              'withinCard' => true
            ), "focus geometry at #{width}px in #{appearance}: #{focus.inspect}"
          end

          find("#{card[:selector]} [data-testid='#{card[:trigger_testid]}']").click
          expect(page).to have_css("[data-testid='#{card[:menu_testid]}']", visible: :visible)

          menu_geometry = action_menu_geometry(card[:menu_testid])
          expect(menu_geometry).to include(
            'menuWithinViewport' => true,
            'itemsWithinViewport' => true,
            'itemContentContained' => true
          ), "menu geometry at #{width}px in #{appearance}: #{menu_geometry.inspect}"
          save_card_screenshot(width: width, appearance: appearance, kind: card[:kind])

          page.send_keys(:escape)
          expect(page.evaluate_script('document.activeElement?.dataset.testid')).to eq(card[:trigger_testid])
        end
      end
    end
  end

  it 'keeps overflow diagnostics privacy-safe', :js do
    visit root_path
    page.execute_script(<<~JS)
      const probe = document.createElement('div');
      probe.textContent = 'fixture name and medication details';
      probe.style.cssText = 'position:absolute; left:100vw; width:40px; height:24px;';
      document.body.appendChild(probe);
    JS

    diagnostics = page.evaluate_script(Rails.root.join('spec/support/mobile_ui_overflowing_elements.js').read)

    expect(diagnostics).to all(
      include('tag', 'id', 'className', 'left', 'right', 'width')
    )
    expect(diagnostics).to all(satisfy { |entry| !entry.key?('text') })
    expect(diagnostics.join).not_to include('fixture name')
    expect(diagnostics.join).not_to include('medication details')
  end

  def page_horizontal_overflow
    page.evaluate_script(<<~JS)
      (() => {
        const width = Math.max(document.documentElement.scrollWidth, document.body.scrollWidth);
        return width - document.documentElement.clientWidth;
      })()
    JS
  end

  def apply_appearance(appearance)
    visit root_path
    page.execute_script(<<~JS)
      localStorage.setItem("med-tracker-appearance", "#{appearance}");
      document.documentElement.classList.toggle("dark", "#{appearance}" === "dark");
      document.documentElement.dataset.appearance = "#{appearance}";
    JS
  end

  def card_action_geometry(cards)
    page.evaluate_script(
      MobileOverflowCardGeometry::CARD_ACTION_GEOMETRY_SCRIPT.sub('__CARDS__', cards.to_json)
    )
  end

  def focus_geometry(selector, card_selector)
    page.evaluate_script(
      MobileOverflowFocusGeometry::FOCUS_GEOMETRY_SCRIPT
        .sub('__SELECTOR__', selector.to_json)
        .sub('__CARD_SELECTOR__', card_selector.to_json)
    )
  end

  def save_card_screenshot(width:, appearance:, kind:)
    return unless [320, 390].include?(width)

    filename = "ui-sweep-card-fixed-#{kind}-#{width}-#{appearance}-menu.png"
    save_ui_screenshot(filename)
  end

  def save_page_screenshot(width:, appearance:)
    return unless [390, 1280].include?(width)

    filename = "ui-sweep-card-fixed-page-#{width}-#{appearance}-closed.png"
    save_ui_screenshot(filename)
  end

  def save_ui_screenshot(filename)
    path = File.join(Capybara.save_path, filename)
    FileUtils.mkdir_p(File.dirname(path))
    page.driver.with_playwright_page { |playwright_page| playwright_page.screenshot(path: path) }
  end

  def action_menu_geometry(menu_testid)
    page.evaluate_script(
      MobileOverflowActionMenuGeometry::ACTION_MENU_GEOMETRY_SCRIPT.sub('__MENU_TESTID__', menu_testid)
    )
  end

  def offscreen_header_actions
    page.evaluate_script(offscreen_header_actions_script)
  end

  def offscreen_header_actions_script
    <<~JS
      (() => {
        const viewportWidth = document.documentElement.clientWidth;
        return Array.from(document.querySelectorAll('.medications-index-actions a, .medications-index-actions button'))
          .filter((element) => {
            const styles = window.getComputedStyle(element);
            const rect = element.getBoundingClientRect();
            return styles.display !== 'none' && rect.width > 0 && (rect.left < -1 || rect.right > viewportWidth + 1);
          })
          .map((element) => {
            const rect = element.getBoundingClientRect();
            return {
              selector: element.id ? `#${element.id}` : element.tagName.toLowerCase(),
              role: element.getAttribute('role'),
              left: Math.round(rect.left),
              right: Math.round(rect.right),
              width: Math.round(rect.width),
              height: Math.round(rect.height),
              viewport: { width: window.innerWidth, height: window.innerHeight }
            };
          });
      })()
    JS
  end

  def create_household_audit_version(admin, event: 'update')
    PaperTrail::Version.create!(
      household_id: browser_household.id,
      actor_membership_id: browser_membership&.id,
      item_type: 'User',
      item_id: admin.id,
      event: event,
      whodunnit: admin.id.to_s,
      created_at: Time.current
    )
  end
end

if ('serviceWorker' in navigator) {
  navigator.serviceWorker.register('/sw.js', { scope: '/' });
}

const navigation = document.querySelector('#dashboard-navigation');
const menuTrigger = document.querySelector('#dashboard-menu-trigger');
const sidebar = document.querySelector('.dashboard-sidebar');
if (navigation && menuTrigger && sidebar) {
  const desktopParent = sidebar.parentElement;
  const desktopSibling = sidebar.nextElementSibling;
  const mobile = matchMedia('(max-width: 767px)');
  const resetMenu = () => {
    menuTrigger.setAttribute('aria-expanded', 'false');
    document.documentElement.classList.remove('dashboard-menu-open');
  };
  const closeMenu = () => {
    navigation.close();
    resetMenu();
  };
  const syncNavigation = () => {
    closeMenu();
    if (mobile.matches) navigation.append(sidebar);
    else desktopParent.insertBefore(sidebar, desktopSibling);
  };
  menuTrigger.addEventListener('click', () => {
    navigation.showModal();
    menuTrigger.setAttribute('aria-expanded', 'true');
    document.documentElement.classList.add('dashboard-menu-open');
  });
  navigation.querySelector('[aria-label="Close menu"]').addEventListener('click', closeMenu);
  navigation.addEventListener('click', event => {
    const bounds = navigation.getBoundingClientRect();
    if (event.clientX < bounds.left || event.clientX > bounds.right || event.clientY < bounds.top || event.clientY > bounds.bottom) closeMenu();
  });
  navigation.addEventListener('keydown', event => {
    if (event.key !== 'Tab') return;
    const controls = [...navigation.querySelectorAll('a[href], button, input:not([type="hidden"]), select, textarea, [tabindex]')]
      .filter(element => !element.disabled && element.tabIndex >= 0 && element.getClientRects().length);
    const first = controls[0];
    const last = controls[controls.length - 1];
    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first.focus();
    }
  });
  navigation.addEventListener('cancel', event => {
    event.preventDefault();
    closeMenu();
  });
  navigation.addEventListener('close', resetMenu);
  mobile.addEventListener('change', syncNavigation);
  syncNavigation();
}

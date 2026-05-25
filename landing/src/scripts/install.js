const COMMANDS = {
  linux: 'curl -fsSL https://raw.githubusercontent.com/xinnaider/orbit/master/scripts/install-linux.sh | bash',
  mac: 'curl -fsSL https://raw.githubusercontent.com/xinnaider/orbit/master/scripts/install-macos.sh | bash',
  win: 'irm https://raw.githubusercontent.com/xinnaider/orbit/master/scripts/install-windows.ps1 | iex',
};

const LABELS = {
  linux: 'Linux selected',
  mac: 'macOS selected',
  win: 'Windows selected',
};

const PROMPTS = {
  linux: '$',
  mac: '$',
  win: 'PS>',
};

function detectOS() {
  const platform = navigator.userAgent.toLowerCase();
  if (platform.includes('mac')) return 'mac';
  if (platform.includes('win')) return 'win';
  return 'linux';
}

async function copyText(text) {
  if (navigator.clipboard?.writeText) {
    await navigator.clipboard.writeText(text);
    return;
  }

  const input = document.createElement('textarea');
  input.value = text;
  input.setAttribute('readonly', '');
  input.style.position = 'fixed';
  input.style.opacity = '0';
  document.body.appendChild(input);
  input.select();
  document.execCommand('copy');
  input.remove();
}

export function initInstallControls() {
  const label = document.querySelector('[data-install-label]');
  const prompt = document.querySelector('[data-install-prompt]');
  const command = document.querySelector('[data-install-command]');
  const copyButton = document.querySelector('[data-copy-install]');
  const backToTop = document.querySelector('[data-back-to-top]');
  let currentOS = detectOS();

  function setOS(os) {
    currentOS = os;
    if (label) label.textContent = LABELS[os];
    if (prompt) prompt.textContent = PROMPTS[os];
    if (command) command.textContent = COMMANDS[os];
    document.querySelectorAll('[data-os]').forEach((item) => {
      item.classList.toggle('is-active', item.getAttribute('data-os') === os);
      item.setAttribute('aria-pressed', String(item.getAttribute('data-os') === os));
    });
  }

  document.querySelectorAll('[data-os]').forEach((item) => {
    item.addEventListener('click', () => setOS(item.getAttribute('data-os')));
  });

  copyButton?.addEventListener('click', async () => {
    try {
      await copyText(COMMANDS[currentOS]);
      copyButton.setAttribute('data-state', 'copied');
      setTimeout(() => copyButton.removeAttribute('data-state'), 1500);
    } catch {
      copyButton.setAttribute('data-state', 'failed');
      setTimeout(() => copyButton.removeAttribute('data-state'), 1500);
    }
  });

  backToTop?.addEventListener('click', () => {
    const lenis = window.__lenis;
    if (lenis) {
      lenis.scrollTo(0, { duration: 0.9 });
      return;
    }
    window.scrollTo({ behavior: 'smooth', top: 0 });
  });

  setOS(currentOS);
}

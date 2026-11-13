// Hand-written progressive enhancements. Every page works without JavaScript;
// this file only adds conveniences on top of server-rendered HTML.
(function () {
  'use strict';

  const TOAST_KEY = 'kiosco:toast';

  function showToast(text) {
    const el = document.createElement('div');
    el.setAttribute('role', 'status');
    el.className = 'fixed top-6 left-1/2 z-[100] -translate-x-1/2 rounded-2xl bg-gray-900 px-5 py-3 text-[15px] font-semibold text-white shadow-xl transition-opacity duration-300';
    el.textContent = text;
    document.body.appendChild(el);
    fadeOut(el);
  }

  function fadeOut(el) {
    setTimeout(() => { el.style.opacity = '0'; }, 1800);
    setTimeout(() => el.remove(), 2200);
  }

  // Toast rendered by the server (`?ok=`): fade out and drop the parameter so a
  // reload does not show it again.
  const toast = document.querySelector('[data-toast]');
  if (toast) {
    fadeOut(toast);
    const url = new URL(location.href);
    if (url.searchParams.has('ok')) {
      url.searchParams.delete('ok');
      history.replaceState(null, '', url);
    }
  }

  // Toast carried over after going back in history (see form submission below).
  try {
    const pending = sessionStorage.getItem(TOAST_KEY);
    if (pending) {
      sessionStorage.removeItem(TOAST_KEY);
      showToast(pending);
    }
  } catch (_) { /* storage unavailable */ }

  // Some browsers (Safari) restore pages from the back-forward cache even with
  // `Cache-Control: no-store`; reload so data is never stale after going back.
  window.addEventListener('pageshow', (event) => {
    if (event.persisted) location.reload();
  });

  // Links with data-replace swap the current history entry (e.g. "Editar"),
  // so going back does not step through every intermediate state.
  document.addEventListener('click', (event) => {
    const link = event.target.closest('a[data-replace]');
    if (!link || event.ctrlKey || event.metaKey || event.shiftKey) return;
    event.preventDefault();
    location.replace(link.href);
  });

  // POST forms: confirm if asked, block double submits and keep the history
  // like a native app. The server answers with a redirect (PRG); instead of
  // pushing a new entry we either replace the current one (same screen) or go
  // back when the destination is the screen we came from.
  const withoutOk = (url) => {
    const copy = new URL(url, location.href);
    copy.searchParams.delete('ok');
    return copy;
  };

  document.addEventListener('submit', async (event) => {
    const form = event.target;
    const message = form.dataset.confirm;
    if (message && !confirm(message)) {
      event.preventDefault();
      return;
    }
    if (form.method.toLowerCase() !== 'post') return;

    event.preventDefault();
    if (form.dataset.sent) return;
    form.dataset.sent = '1';
    const body = new URLSearchParams(new FormData(form));
    form.querySelectorAll('button[type="submit"]').forEach((button) => {
      button.disabled = true;
      button.classList.add('opacity-60');
    });

    let response;
    try {
      response = await fetch(form.action, { method: 'POST', body, credentials: 'same-origin' });
    } catch (_) {
      // Network failure: fall back to a regular submission.
      delete form.dataset.sent;
      form.submit();
      return;
    }

    const html = await response.text();
    if (!response.ok || !response.redirected) {
      // Error page (or unexpected answer): show it in place.
      document.open();
      document.write(html);
      document.close();
      return;
    }

    const target = new URL(response.url);
    const here = withoutOk(location.href);
    const referrer = document.referrer ? withoutOk(document.referrer) : null;

    if (target.pathname !== here.pathname && referrer && referrer.origin === here.origin
        && referrer.pathname === target.pathname) {
      const text = new DOMParser().parseFromString(html, 'text/html').querySelector('[data-toast]');
      try { if (text) sessionStorage.setItem(TOAST_KEY, text.textContent.trim()); } catch (_) { /* ignore */ }
      history.back();
    } else {
      location.replace(target.href);
    }
  });

  // <button data-back> goes back in history.
  document.querySelectorAll('[data-back]').forEach((button) => {
    button.addEventListener('click', () => history.back());
  });

  // <select data-navigate>: option values are URLs.
  document.querySelectorAll('select[data-navigate]').forEach((select) => {
    select.addEventListener('change', () => { location.href = select.value; });
  });

  // Bottom navigation hides while scrolling down.
  const bottomNav = document.getElementById('bottom-nav');
  if (bottomNav) {
    let lastY = 0;
    let ticking = false;
    window.addEventListener('scroll', () => {
      if (ticking) return;
      ticking = true;
      requestAnimationFrame(() => {
        const current = window.scrollY;
        if (current > lastY && current > 50) bottomNav.style.transform = 'translateY(calc(100% + 40px))';
        else if (current < lastY) bottomNav.style.transform = '';
        lastY = Math.max(current, 0);
        ticking = false;
      });
    }, { passive: true });
  }

  // Student list: free-text search + grade pills.
  const filter = document.querySelector('[data-student-filter]');
  if (filter) {
    const rows = filter.querySelectorAll('[data-row]');
    const pills = filter.querySelectorAll('[data-filter-grade]');
    const search = filter.querySelector('[data-filter-search]');
    const count = filter.querySelector('[data-filter-count]');
    const empty = filter.querySelector('[data-filter-empty]');
    const normalize = (text) => text.normalize('NFD').replace(/[̀-ͯ]/g, '').toLowerCase();
    let grade = '';

    const apply = () => {
      const query = normalize(search.value.trim());
      let visible = 0;
      rows.forEach((row) => {
        const show = (!grade || row.dataset.grade === grade) && (!query || normalize(row.dataset.name).includes(query));
        row.classList.toggle('hidden', !show);
        if (show) visible++;
      });
      count.textContent = 'encontrados: ' + visible;
      empty.classList.toggle('hidden', visible > 0);
    };

    pills.forEach((pill) => pill.addEventListener('click', () => {
      grade = pill.dataset.filterGrade;
      pills.forEach((p) => p.classList.toggle('pill-active', p === pill));
      apply();
    }));
    search.addEventListener('input', apply);
    apply();
  }

  // Consumption editor: +/- steppers and live total.
  const editor = document.querySelector('[data-consumo-editor]');
  if (editor) {
    const total = editor.querySelector('[data-total]');
    const items = [...editor.querySelectorAll('[data-product]')];

    const render = () => {
      let sum = 0;
      items.forEach((item) => {
        const input = item.querySelector('[data-qty]');
        const qty = Math.min(999, Math.max(0, parseInt(input.value, 10) || 0));
        sum += qty * Number(item.dataset.price);
        input.classList.toggle('text-gray-400', qty === 0);
        item.querySelector('[data-step="-1"]').classList.toggle('opacity-20', qty === 0);
      });
      total.textContent = sum.toFixed(2);
    };

    editor.addEventListener('click', (event) => {
      const button = event.target.closest('[data-step]');
      if (!button) return;
      const input = button.closest('[data-product]').querySelector('[data-qty]');
      const next = (parseInt(input.value, 10) || 0) + Number(button.dataset.step);
      input.value = Math.min(999, Math.max(0, next));
      render();
    });
    editor.addEventListener('input', render);
    render();
  }

  // <button data-share-text="...">: share a text message (WhatsApp, SMS…).
  // Falls back to WhatsApp Web where the Web Share API is not available.
  document.addEventListener('click', async (event) => {
    const button = event.target.closest('button[data-share-text]');
    if (!button) return;
    const text = button.dataset.shareText;
    if (navigator.share) {
      try { await navigator.share({ text }); } catch (_) { /* cancelled */ }
    } else {
      window.open('https://wa.me/?text=' + encodeURIComponent(text), '_blank', 'noopener');
    }
  });

  // Weekly receipt rendered as a PNG image.
  const receipt = document.getElementById('comprobante');
  const receiptBlob = async () => {
    const canvas = await html2canvas(receipt, { scale: 2, backgroundColor: '#ffffff' });
    return new Promise((resolve) => canvas.toBlob(resolve, 'image/png'));
  };

  // Share the receipt image directly (mobile share sheet → WhatsApp, etc.).
  const shareButton = document.querySelector('[data-share-receipt]');
  if (shareButton && receipt) {
    const probe = new File([''], 'probe.png', { type: 'image/png' });
    if (navigator.canShare && navigator.canShare({ files: [probe] })) {
      shareButton.hidden = false;
      const label = shareButton.querySelector('span');
      shareButton.addEventListener('click', async () => {
        if (typeof html2canvas !== 'function') return;
        label.textContent = 'Generando…';
        shareButton.disabled = true;
        try {
          const file = new File([await receiptBlob()], shareButton.dataset.fileName, { type: 'image/png' });
          await navigator.share({ files: [file], text: shareButton.dataset.caption });
        } catch (_) { /* cancelled */ }
        label.textContent = '📤 Compartir';
        shareButton.disabled = false;
      });
    }
  }

  // Weekly receipt: copy as PNG (download when clipboard images are unsupported).
  const copyButton = document.querySelector('[data-copy-receipt]');
  if (copyButton && receipt) {
    const label = copyButton.querySelector('span');
    const setState = (text, busy) => { label.textContent = text; copyButton.disabled = busy; };

    copyButton.addEventListener('click', async () => {
      if (typeof html2canvas !== 'function') { setState('✗ Error', false); return; }
      setState('Generando…', true);
      try {
        const blob = await receiptBlob();
        if (navigator.clipboard && window.ClipboardItem) {
          await navigator.clipboard.write([new ClipboardItem({ 'image/png': blob })]);
          setState('✓ Copiado', true);
        } else {
          const link = document.createElement('a');
          link.href = URL.createObjectURL(blob);
          link.download = copyButton.dataset.fileName || 'nota-venta.png';
          link.click();
          setTimeout(() => URL.revokeObjectURL(link.href), 1000);
          setState('✓ Descargado', true);
        }
      } catch (error) {
        console.error(error);
        setState('✗ Error', true);
      }
      setTimeout(() => setState('📋 Copiar', false), 2000);
    });
  }
})();

// Hand-written progressive enhancements. Every page works without JavaScript;
// this file only adds conveniences on top of server-rendered HTML.
(function () {
  'use strict';

  // Toast: fade out and drop `?ok=` so a reload does not show it again.
  const toast = document.querySelector('[data-toast]');
  if (toast) {
    setTimeout(() => { toast.style.opacity = '0'; }, 1800);
    setTimeout(() => toast.remove(), 2200);
    const url = new URL(location.href);
    if (url.searchParams.has('ok')) {
      url.searchParams.delete('ok');
      history.replaceState(null, '', url);
    }
  }

  // <form data-confirm="..."> asks before submitting.
  document.addEventListener('submit', (event) => {
    const message = event.target.dataset.confirm;
    if (message && !confirm(message)) event.preventDefault();
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

  // Weekly receipt: copy as PNG (download when clipboard images are unsupported).
  const copyButton = document.querySelector('[data-copy-receipt]');
  if (copyButton) {
    const label = copyButton.querySelector('span');
    const receipt = document.getElementById('comprobante');
    const setState = (text, busy) => { label.textContent = text; copyButton.disabled = busy; };

    copyButton.addEventListener('click', async () => {
      if (typeof html2canvas !== 'function') { setState('✗ Error', false); return; }
      setState('Generando…', true);
      try {
        const canvas = await html2canvas(receipt, { scale: 2, backgroundColor: '#ffffff' });
        const blob = await new Promise((resolve) => canvas.toBlob(resolve, 'image/png'));
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

(function () {
  'use strict';

  const tabsList = document.getElementById('tabs-list');
  const btnTabsLeft = document.getElementById('btn-tabs-left');
  const btnTabsRight = document.getElementById('btn-tabs-right');
  const welcomeView = document.getElementById('welcome-view');
  const workspace = document.getElementById('workspace');
  const tocSidebar = document.getElementById('toc-sidebar');
  const tabOutline = document.getElementById('tab-outline');
  const tabFiles = document.getElementById('tab-files');
  const tocNav = document.getElementById('toc-nav');
  const filesPanel = document.getElementById('files-panel');
  const filesFolderName = document.getElementById('files-folder-name');
  const filesFilterWrap = document.getElementById('files-filter-wrap');
  const filesFilter = document.getElementById('files-filter');
  const filesTree = document.getElementById('files-tree');
  const btnFilesRefresh = document.getElementById('btn-files-refresh');
  const btnFilesOpen = document.getElementById('btn-files-open');
  const btnOpenFolder = document.getElementById('btn-open-folder');
  const readerScroll = document.getElementById('reader-scroll');
  const mdBody = document.getElementById('markdown-body');
  const docMeta = document.getElementById('doc-meta');
  const docStatsText = document.getElementById('doc-stats-text');
  const liveBadge = document.getElementById('live-badge');
  const liveText = document.getElementById('live-text');
  const btnToc = document.getElementById('btn-toc');
  const btnFind = document.getElementById('btn-find');
  const btnZoomReset = document.getElementById('btn-zoom-reset');
  const iconSun = document.getElementById('icon-sun');
  const iconMoon = document.getElementById('icon-moon');
  const findBar = document.getElementById('find-bar');
  const findInput = document.getElementById('find-input');
  const findCount = document.getElementById('find-count');
  const dropOverlay = document.getElementById('drop-overlay');
  const startupLoader = document.getElementById('startup-loader');

  let loaderHidden = false;
  window.__MDV_HIDE_LOADER__ = function () {
    if (loaderHidden) return;
    loaderHidden = true;
    if (!startupLoader) return;
    requestAnimationFrame(() => {
      startupLoader.classList.add('fade-out');
      setTimeout(() => startupLoader.remove(), 200);
    });
  };
  setTimeout(() => window.__MDV_HIDE_LOADER__(), 2500);

  const state = {
    tabs: [], // { id, kind: 'new_tab' | 'doc', doc: RenderedDoc | null, scrollTop: number }
    activeTabId: null,
    tocVisible: true,
    sidebarTab: 'outline',
    folder: null,
    collapsedDirs: new Set(),
    dark: true,
    zoom: 100,
    hits: [],
    hitIdx: -1,
    restoring: false,
  };

  function nextId() {
    return 'tab-' + Date.now() + '-' + Math.random().toString(36).slice(2, 7);
  }

  function postIpc(payload) {
    if (window.ipc && typeof window.ipc.postMessage === 'function') {
      window.ipc.postMessage(JSON.stringify(payload));
    }
  }

  // ── Session Persistence ────────────────────────────────────────
  function buildSessionPayload() {
    const cur = activeTab();
    if (cur && cur.kind === 'doc') {
      cur.scrollTop = readerScroll.scrollTop;
    }

    const activeIdx = Math.max(
      0,
      state.tabs.findIndex((t) => t.id === state.activeTabId)
    );

    const serializedTabs = state.tabs.map((t) => {
      if (t.kind === 'new_tab' || !t.doc) {
        return { kind: 'new_tab' };
      }
      if (t.doc.path === 'Welcome-Demo.md') {
        return { kind: 'demo', scroll_top: t.scrollTop || 0 };
      }
      return { kind: 'file', path: t.doc.path, scroll_top: t.scrollTop || 0 };
    });

    if (serializedTabs.length === 0) {
      serializedTabs.push({ kind: 'new_tab' });
    }

    return {
      tabs: serializedTabs,
      active_index: Math.min(activeIdx, serializedTabs.length - 1),
      dark: state.dark,
      toc_visible: state.tocVisible,
      zoom: state.zoom,
      open_folder: state.folder ? state.folder.root_path : null,
      sidebar_tab: state.sidebarTab,
    };
  }

  let saveTimer = null;
  function saveSessionNow() {
    if (state.restoring) return;
    clearTimeout(saveTimer);
    saveTimer = null;
    postIpc({ type: 'save_session', session: buildSessionPayload() });
  }

  function scheduleSaveSession() {
    if (state.restoring) return;
    clearTimeout(saveTimer);
    saveTimer = setTimeout(() => {
      saveTimer = null;
      postIpc({ type: 'save_session', session: buildSessionPayload() });
    }, 300);
  }

  // ── Theme & Zoom ───────────────────────────────────────────────
  function applyTheme(dark, notifyHost = true) {
    state.dark = dark;
    const bg = dark ? '#0D1017' : '#FFFFFF';
    document.documentElement.setAttribute('data-theme', dark ? 'dark' : 'light');
    document.documentElement.style.background = bg;
    document.body.style.background = bg;
    iconSun.classList.toggle('hidden', !dark);
    iconMoon.classList.toggle('hidden', dark);
    if (notifyHost) {
      postIpc({ type: 'set_theme', dark });
      saveSessionNow();
    }
    reRenderMermaidForTheme();
  }

  function applyZoom(pct, persist = true) {
    state.zoom = Math.max(70, Math.min(200, pct));
    const pxSize = (15.5 * state.zoom) / 100;
    mdBody.style.setProperty('--doc-font-size', `${pxSize.toFixed(2)}px`);
    btnZoomReset.textContent = `${state.zoom}%`;
    if (persist) saveSessionNow();
  }

  // ── Tabs Management ────────────────────────────────────────────
  function findTabByPath(path) {
    return state.tabs.find((t) => t.kind === 'doc' && t.doc && t.doc.path === path) || null;
  }

  function activeTab() {
    return state.tabs.find((t) => t.id === state.activeTabId) || null;
  }

  function updateTabScrollButtons() {
    if (!btnTabsLeft || !btnTabsRight) return;
    const overflow = tabsList.scrollWidth > tabsList.clientWidth + 2;
    btnTabsLeft.classList.toggle('hidden', !overflow);
    btnTabsRight.classList.toggle('hidden', !overflow);
    if (overflow) {
      btnTabsLeft.classList.toggle('disabled', tabsList.scrollLeft <= 1);
      btnTabsRight.classList.toggle(
        'disabled',
        tabsList.scrollLeft + tabsList.clientWidth >= tabsList.scrollWidth - 2
      );
    }
  }

  function renderTabs() {
    tabsList.innerHTML = '';
    let activeBtn = null;
    for (const tab of state.tabs) {
      const btn = document.createElement('button');
      btn.type = 'button';
      const isActive = tab.id === state.activeTabId;
      btn.className = 'tab-item' + (isActive ? ' active' : '');
      btn.title = tab.kind === 'doc' && tab.doc ? tab.doc.path : 'New Tab';

      const title = document.createElement('span');
      title.className = 'tab-title';
      title.textContent = tab.kind === 'doc' && tab.doc ? tab.doc.filename : 'New Tab';

      const close = document.createElement('span');
      close.className = 'tab-close';
      close.textContent = '×';
      close.title = 'Close tab (Ctrl+W)';
      close.addEventListener('click', (e) => {
        e.stopPropagation();
        closeTab(tab.id);
      });

      btn.addEventListener('click', () => activateTab(tab.id));
      btn.addEventListener('auxclick', (e) => {
        if (e.button === 1) closeTab(tab.id);
      });

      btn.appendChild(title);
      btn.appendChild(close);
      tabsList.appendChild(btn);
      if (isActive) activeBtn = btn;
    }

    requestAnimationFrame(() => {
      if (activeBtn) {
        activeBtn.scrollIntoView({ inline: 'nearest', block: 'nearest', behavior: 'smooth' });
      }
      updateTabScrollButtons();
    });
  }

  function updateSidebarVisibility() {
    const tab = activeTab();
    const isDoc = Boolean(tab && tab.kind === 'doc' && tab.doc);
    const showSidebar =
      state.tocVisible &&
      ((isDoc && (!tab.doc.is_mermaid || state.sidebarTab === 'files')) || Boolean(state.folder));

    tocSidebar.classList.toggle('hidden', !showSidebar);
    btnToc.classList.toggle('active', state.tocVisible);

    const isFiles = state.sidebarTab === 'files';
    tabOutline.classList.toggle('active', !isFiles);
    tabOutline.setAttribute('aria-selected', String(!isFiles));
    tabFiles.classList.toggle('active', isFiles);
    tabFiles.setAttribute('aria-selected', String(isFiles));

    tocNav.classList.toggle('hidden', isFiles);
    filesPanel.classList.toggle('hidden', !isFiles);
    docMeta.classList.toggle('hidden', !isDoc);
  }

  function setSidebarTab(mode, persist = true) {
    state.sidebarTab = mode === 'files' ? 'files' : 'outline';
    updateSidebarVisibility();
    if (persist) saveSessionNow();
  }

  function activateTab(tabId, persist = true) {
    const prev = activeTab();
    if (prev && prev.kind === 'doc') {
      prev.scrollTop = readerScroll.scrollTop;
    }

    state.activeTabId = tabId;
    const next = activeTab();
    if (!next || next.kind === 'new_tab' || !next.doc) {
      welcomeView.classList.remove('hidden');
      readerScroll.classList.add('hidden');
      document.title = 'MDViewerByAli';
      updateSidebarVisibility();
      renderFileTree();
      renderTabs();
      if (persist) saveSessionNow();
      return;
    }

    welcomeView.classList.add('hidden');
    readerScroll.classList.remove('hidden');
    renderTabs();
    renderDocumentIntoWorkspace(next.doc, next.scrollTop);
    renderFileTree();
    if (persist) saveSessionNow();
  }

  function createNewTab() {
    const tab = { id: nextId(), kind: 'new_tab', doc: null, scrollTop: 0 };
    state.tabs.push(tab);
    activateTab(tab.id);
  }

  function closeTab(tabId) {
    const idx = state.tabs.findIndex((t) => t.id === tabId);
    if (idx < 0) return;
    const [removed] = state.tabs.splice(idx, 1);
    if (removed.kind === 'doc' && removed.doc && removed.doc.path !== 'Welcome-Demo.md') {
      const stillOpen = state.tabs.some(
        (t) => t.kind === 'doc' && t.doc && t.doc.path === removed.doc.path
      );
      if (!stillOpen) {
        postIpc({ type: 'close_tab', path: removed.doc.path });
      }
    }

    if (state.tabs.length === 0) {
      const blank = { id: nextId(), kind: 'new_tab', doc: null, scrollTop: 0 };
      state.tabs.push(blank);
      activateTab(blank.id);
      return;
    }

    if (state.activeTabId === tabId) {
      const fallback = state.tabs[idx] || state.tabs[idx - 1];
      activateTab(fallback.id);
    } else {
      renderTabs();
      saveSessionNow();
    }
  }

  window.__MDV_OPEN_DOC__ = function (doc) {
    const existing = findTabByPath(doc.path);
    if (existing) {
      existing.doc = doc;
      activateTab(existing.id);
      return;
    }

    const current = activeTab();
    if (current && current.kind === 'new_tab') {
      current.kind = 'doc';
      current.doc = doc;
      current.scrollTop = 0;
      activateTab(current.id);
      return;
    }

    const tab = { id: nextId(), kind: 'doc', doc, scrollTop: 0 };
    state.tabs.push(tab);
    activateTab(tab.id);
  };

  window.__MDV_UPDATE_DOC__ = function (doc) {
    const existing = findTabByPath(doc.path);
    if (!existing) return;
    existing.doc = doc;
    if (existing.id === state.activeTabId) {
      const keepScroll = readerScroll.scrollTop;
      renderDocumentIntoWorkspace(doc, keepScroll);
    }
  };

  window.__MDV_RESTORE_SESSION__ = function (payload) {
    state.restoring = true;
    try {
      applyTheme(payload.dark !== false, false);
      state.tocVisible = payload.toc_visible !== false;
      state.sidebarTab = payload.sidebar_tab === 'files' ? 'files' : 'outline';
      state.folder = payload.folder || null;
      applyZoom(payload.zoom || 100, false);

      state.tabs = [];
      const incoming = Array.isArray(payload.tabs) ? payload.tabs : [];
      for (const item of incoming) {
        if (item.kind === 'doc' && item.doc) {
          state.tabs.push({
            id: nextId(),
            kind: 'doc',
            doc: item.doc,
            scrollTop: item.scroll_top || 0,
          });
        } else {
          state.tabs.push({ id: nextId(), kind: 'new_tab', doc: null, scrollTop: 0 });
        }
      }

      if (state.tabs.length === 0) {
        state.tabs.push({ id: nextId(), kind: 'new_tab', doc: null, scrollTop: 0 });
      }

      const idx = Math.max(0, Math.min(payload.active_index || 0, state.tabs.length - 1));
      activateTab(state.tabs[idx].id, false);
      renderFileTree();
      updateSidebarVisibility();
    } finally {
      state.restoring = false;
    }
  };

  window.__MDV_SET_FOLDER__ = function (tree) {
    const prevRoot = state.folder ? state.folder.root_path : null;
    if (!tree || tree.root_path !== prevRoot) {
      state.collapsedDirs.clear();
    }
    state.folder = tree || null;
    state.tocVisible = true;
    state.sidebarTab = 'files';
    renderFileTree();
    updateSidebarVisibility();
    saveSessionNow();
  };

  window.__MDV_DRAG__ = function (active) {
    dropOverlay.classList.toggle('hidden', !active);
  };

  // ── Document Rendering ─────────────────────────────────────────
  function renderDocumentIntoWorkspace(doc, restoreScrollTop) {
    document.title = `${doc.filename} — MDViewerByAli`;
    mdBody.innerHTML = doc.html;

    updateSidebarVisibility();

    const isDemo = doc.path === 'Welcome-Demo.md';
    liveBadge.classList.toggle('paused', isDemo);
    liveText.textContent = isDemo ? 'Demo' : 'Live';
    docStatsText.textContent = `${doc.stats.words} words · ${doc.stats.read_minutes}m · ${doc.stats.size_label}`;

    buildToC(doc.toc);
    highlightCodeBlocks();
    initMath();
    initMermaid();

    requestAnimationFrame(() => {
      readerScroll.scrollTop = restoreScrollTop || 0;
      updateActiveToc();
    });

    if (!findBar.classList.contains('hidden') && findInput.value.trim()) {
      runFind(findInput.value);
    }
  }

  const loadedScripts = new Map();
  function loadScriptOnce(src) {
    if (loadedScripts.has(src)) return loadedScripts.get(src);
    const promise = new Promise((resolve, reject) => {
      const s = document.createElement('script');
      s.src = src;
      s.async = true;
      s.onload = () => resolve();
      s.onerror = (err) => {
        loadedScripts.delete(src);
        reject(err);
      };
      document.head.appendChild(s);
    });
    loadedScripts.set(src, promise);
    return promise;
  }

  const loadedStyles = new Map();
  function loadCssOnce(href) {
    if (loadedStyles.has(href)) return loadedStyles.get(href);
    const promise = new Promise((resolve, reject) => {
      const link = document.createElement('link');
      link.rel = 'stylesheet';
      link.href = href;
      link.onload = () => resolve();
      link.onerror = (err) => {
        loadedStyles.delete(href);
        reject(err);
      };
      document.head.appendChild(link);
    });
    loadedStyles.set(href, promise);
    return promise;
  }

  function initMath() {
    const els = Array.from(mdBody.querySelectorAll('.mdv-math-inline, .mdv-math-display'));
    if (els.length === 0) return;

    els.forEach((el) => {
      if (!el.dataset.tex) {
        el.dataset.tex = el.textContent;
      }
    });

    const renderNow = () => {
      if (typeof katex === 'undefined') return;
      for (const el of els) {
        if (!el.isConnected) continue;
        const tex = el.dataset.tex || el.textContent;
        const displayMode = el.classList.contains('mdv-math-display');
        try {
          katex.render(tex, el, {
            displayMode,
            throwOnError: false,
            strict: 'ignore',
          });
        } catch (_) {
          el.classList.add('mdv-math-error');
        }
      }
    };

    if (typeof katex !== 'undefined') {
      loadCssOnce('/vendor/katex/katex.min.css').catch(() => {});
      renderNow();
    } else {
      Promise.all([
        loadCssOnce('/vendor/katex/katex.min.css'),
        loadScriptOnce('/vendor/katex/katex.min.js'),
      ])
        .then(renderNow)
        .catch(() => {});
    }
  }

  function highlightCodeBlocks() {
    mdBody.querySelectorAll('.copy-btn').forEach((btn) => {
      btn.addEventListener('click', async () => {
        const code = btn.closest('.code-block')?.querySelector('pre code')?.textContent || '';
        try {
          await navigator.clipboard.writeText(code);
          btn.textContent = 'Copied!';
          setTimeout(() => { btn.textContent = 'Copy'; }, 1400);
        } catch (_) {}
      });
    });

    const blocks = Array.from(mdBody.querySelectorAll('pre code'));
    if (blocks.length === 0) return;

    const runHighlight = () => {
      if (typeof hljs === 'undefined') return;
      for (const block of blocks) {
        if (!block.isConnected) continue;
        try {
          hljs.highlightElement(block);
        } catch (_) {}
      }
    };

    if (typeof hljs !== 'undefined') {
      runHighlight();
    } else {
      loadScriptOnce('/vendor/highlight.min.js').then(runHighlight).catch(() => {});
    }
  }

  // ── ToC & Scroll Sync ──────────────────────────────────────────
  let tocPairs = [];

  function buildToC(toc) {
    tocNav.innerHTML = '';
    tocPairs = [];

    if (!toc || toc.length === 0) {
      const empty = document.createElement('p');
      empty.className = 'toc-empty';
      empty.textContent = 'No headings';
      tocNav.appendChild(empty);
      return;
    }

    for (const item of toc) {
      const a = document.createElement('a');
      a.className = `toc-item toc-h${item.level}`;
      a.href = '#' + item.id;
      a.title = item.text;
      a.style.paddingLeft = `${(item.level - 1) * 12 + 10}px`;

      const bullet = document.createElement('span');
      bullet.className = 'toc-bullet';
      bullet.textContent = '•';

      const label = document.createElement('span');
      label.className = 'toc-label';
      label.textContent = item.text;

      a.appendChild(bullet);
      a.appendChild(label);

      a.addEventListener('click', (e) => {
        e.preventDefault();
        const target = document.getElementById(item.id);
        if (target) {
          const top = target.offsetTop - 20;
          readerScroll.scrollTo({ top, behavior: 'smooth' });
        }
      });

      tocNav.appendChild(a);
      const headingEl = document.getElementById(item.id);
      if (headingEl) {
        tocPairs.push({ headingEl, link: a });
      }
    }
  }

  let activeTocLink = null;
  function updateActiveToc() {
    if (tocPairs.length === 0) return;

    const nearBottom =
      readerScroll.scrollTop + readerScroll.clientHeight >= readerScroll.scrollHeight - 6;

    let best = tocPairs[0].link;
    if (nearBottom) {
      best = tocPairs[tocPairs.length - 1].link;
    } else {
      const containerTop = readerScroll.getBoundingClientRect().top;
      for (const pair of tocPairs) {
        if (pair.headingEl.getBoundingClientRect().top - containerTop <= 72) {
          best = pair.link;
        }
      }
    }

    if (best !== activeTocLink) {
      if (activeTocLink) activeTocLink.classList.remove('toc-active');
      activeTocLink = best;
      if (activeTocLink) {
        activeTocLink.classList.add('toc-active');
        activeTocLink.scrollIntoView({ block: 'nearest' });
      }
    }
  }

  let scrollTicking = false;
  readerScroll.addEventListener('scroll', () => {
    const cur = activeTab();
    if (cur && cur.kind === 'doc') {
      cur.scrollTop = readerScroll.scrollTop;
      scheduleSaveSession();
    }
    if (!scrollTicking) {
      scrollTicking = true;
      requestAnimationFrame(() => {
        updateActiveToc();
        scrollTicking = false;
      });
    }
  }, { passive: true });

  // ── Mermaid Diagrams & Lightbox ────────────────────────────────
  function initMermaid() {
    const els = Array.from(mdBody.querySelectorAll('.mermaid'));
    if (els.length === 0) return;

    els.forEach((el) => {
      if (!el.dataset.src) {
        el.dataset.src = el.textContent.trim();
      }
      el.removeAttribute('data-processed');
    });

    const renderNow = () => {
      if (typeof mermaid === 'undefined') return;
      const connected = els.filter((el) => el.isConnected);
      if (connected.length === 0) return;
      connected.forEach((el) => el.classList.remove('mermaid-loading'));
      mermaid.initialize({
        startOnLoad: false,
        securityLevel: 'loose',
        theme: state.dark ? 'dark' : 'default',
      });
      mermaid.run({ nodes: connected });
    };

    if (typeof mermaid !== 'undefined') {
      renderNow();
    } else {
      els.forEach((el) => el.classList.add('mermaid-loading'));
      loadScriptOnce('/vendor/mermaid.min.js')
        .then(renderNow)
        .catch(() => {
          els.forEach((el) => el.classList.remove('mermaid-loading'));
        });
    }
  }

  function reRenderMermaidForTheme() {
    const els = Array.from(mdBody.querySelectorAll('.mermaid'));
    if (els.length === 0) return;
    els.forEach((el) => {
      if (el.dataset.src) {
        el.removeAttribute('data-processed');
        el.textContent = el.dataset.src;
      }
    });

    const renderNow = () => {
      if (typeof mermaid === 'undefined') return;
      const connected = els.filter((el) => el.isConnected);
      if (connected.length === 0) return;
      connected.forEach((el) => el.classList.remove('mermaid-loading'));
      mermaid.initialize({
        startOnLoad: false,
        securityLevel: 'loose',
        theme: state.dark ? 'dark' : 'default',
      });
      mermaid.run({ nodes: connected });
    };

    if (typeof mermaid !== 'undefined') {
      renderNow();
    } else {
      els.forEach((el) => el.classList.add('mermaid-loading'));
      loadScriptOnce('/vendor/mermaid.min.js')
        .then(renderNow)
        .catch(() => {
          els.forEach((el) => el.classList.remove('mermaid-loading'));
        });
    }
  }

  // Interactive Lightbox
  (function setupLightbox() {
    const lb = document.getElementById('md-lightbox');
    const inner = document.getElementById('md-lightbox-inner');
    const closeBtn = document.getElementById('md-lightbox-close');

    let tx = 0, ty = 0, sc = 1;
    let dragging = false, dragX = 0, dragY = 0;
    let wheelTimer = null;

    function applyTransform() {
      inner.style.transform = `translate(${tx}px, ${ty}px) scale(${sc})`;
    }

    function fitToScreen() {
      const svgEl = inner.querySelector('svg');
      const imgEl = inner.querySelector('img');
      const vw = window.innerWidth, vh = window.innerHeight, pad = 64;
      let sw = 0, sh = 0;
      if (svgEl) {
        const vb = svgEl.viewBox && svgEl.viewBox.baseVal;
        sw = (vb && vb.width > 0) ? vb.width : (parseFloat(svgEl.getAttribute('width')) || 800);
        sh = (vb && vb.height > 0) ? vb.height : (parseFloat(svgEl.getAttribute('height')) || 600);
      } else if (imgEl) {
        sw = imgEl.naturalWidth || imgEl.width || 800;
        sh = imgEl.naturalHeight || imgEl.height || 600;
      } else {
        return;
      }
      sc = Math.min((vw - pad * 2) / sw, (vh - pad * 2) / sh);
      if (imgEl) sc = Math.min(sc, 4);
      tx = (vw - sw * sc) / 2;
      ty = (vh - sh * sc) / 2;
      applyTransform();
    }

    function openLightbox(sourceEl) {
      inner.innerHTML = '';
      const clone = sourceEl.cloneNode(true);
      clone.style.maxWidth = 'none';
      clone.style.maxHeight = 'none';
      const isSvg = clone.tagName && clone.tagName.toLowerCase() === 'svg';
      if (isSvg) {
        const vb = clone.viewBox && clone.viewBox.baseVal;
        if (vb && vb.width > 0) {
          clone.setAttribute('width', vb.width);
          clone.setAttribute('height', vb.height);
        }
      } else if (clone.tagName && clone.tagName.toLowerCase() === 'img') {
        clone.removeAttribute('width');
        clone.removeAttribute('height');
        if (!clone.complete) {
          clone.addEventListener('load', fitToScreen, { once: true });
        }
      }
      inner.appendChild(clone);
      lb.classList.add('open');
      lb.setAttribute('aria-hidden', 'false');
      requestAnimationFrame(fitToScreen);
    }

    function closeLightbox() {
      lb.classList.remove('open', 'dragging');
      lb.setAttribute('aria-hidden', 'true');
      dragging = false;
      inner.innerHTML = '';
    }

    closeBtn.addEventListener('click', (e) => {
      e.stopPropagation();
      closeLightbox();
    });
    lb.addEventListener('click', (e) => {
      if (e.target === lb) closeLightbox();
    });
    lb.addEventListener('dblclick', (e) => {
      e.stopPropagation();
      fitToScreen();
    });

    lb.addEventListener('wheel', (e) => {
      e.preventDefault();
      inner.classList.remove('smooth');
      clearTimeout(wheelTimer);
      wheelTimer = setTimeout(() => inner.classList.add('smooth'), 80);
      const factor = e.deltaY < 0 ? 1.1 : 1 / 1.1;
      tx = e.clientX - (e.clientX - tx) * factor;
      ty = e.clientY - (e.clientY - ty) * factor;
      sc = Math.max(0.05, Math.min(sc * factor, 20));
      applyTransform();
    }, { passive: false });

    lb.addEventListener('mousedown', (e) => {
      if (e.button !== 0) return;
      e.preventDefault();
      inner.classList.remove('smooth');
      dragging = true;
      dragX = e.clientX;
      dragY = e.clientY;
      lb.classList.add('dragging');
    });

    document.addEventListener('mousemove', (e) => {
      if (!dragging) return;
      tx += e.clientX - dragX;
      ty += e.clientY - dragY;
      dragX = e.clientX;
      dragY = e.clientY;
      applyTransform();
    });

    document.addEventListener('mouseup', () => {
      if (dragging) {
        dragging = false;
        lb.classList.remove('dragging');
        inner.classList.add('smooth');
      }
    });

    document.addEventListener('keydown', (e) => {
      if (e.key === 'Escape' && lb.classList.contains('open')) {
        e.stopPropagation();
        closeLightbox();
      }
    });

    mdBody.addEventListener('click', (e) => {
      if (lb.classList.contains('open')) return;
      const mermaidEl = e.target.closest && e.target.closest('.mermaid');
      if (mermaidEl) {
        const svgEl = mermaidEl.querySelector('svg');
        if (svgEl) openLightbox(svgEl);
        return;
      }
      const imgEl = e.target.closest && e.target.closest('img');
      if (imgEl && !imgEl.closest('a')) {
        openLightbox(imgEl);
      }
    });
  })();

  // ── Folder File Tree ───────────────────────────────────────────
  function filterTreeNodes(nodes, query) {
    if (!query) return nodes;
    const out = [];
    for (const node of nodes) {
      if (node.is_dir) {
        const filteredChildren = filterTreeNodes(node.children || [], query);
        if (filteredChildren.length > 0 || node.name.toLowerCase().includes(query)) {
          out.push({
            ...node,
            children: filteredChildren.length > 0 ? filteredChildren : node.children,
          });
        }
      } else if (node.name.toLowerCase().includes(query)) {
        out.push(node);
      }
    }
    return out;
  }

  function renderFileTree() {
    if (!filesTree) return;
    filesTree.innerHTML = '';

    const folder = state.folder;
    filesFolderName.textContent = folder ? folder.root_name : 'No folder open';
    filesFolderName.title = folder ? folder.root_path : '';
    btnFilesRefresh.classList.toggle('hidden', !folder);
    filesFilterWrap.classList.toggle('hidden', !folder);

    if (!folder) {
      const empty = document.createElement('div');
      empty.className = 'tree-empty';
      const p = document.createElement('span');
      p.textContent = 'Open a folder to browse Markdown & Mermaid files.';
      const btn = document.createElement('button');
      btn.type = 'button';
      btn.className = 'btn-secondary';
      btn.textContent = 'Open Folder';
      btn.addEventListener('click', () => postIpc({ type: 'open_folder_dialog' }));
      empty.appendChild(p);
      empty.appendChild(btn);
      filesTree.appendChild(empty);
      return;
    }

    const query = (filesFilter.value || '').trim().toLowerCase();
    const visibleNodes = filterTreeNodes(folder.nodes || [], query);
    if (visibleNodes.length === 0) {
      const empty = document.createElement('div');
      empty.className = 'tree-empty';
      empty.textContent = query ? 'No matching files' : 'No supported files found in this folder';
      filesTree.appendChild(empty);
      return;
    }

    const cur = activeTab();
    const activePath =
      cur && cur.kind === 'doc' && cur.doc && cur.doc.path
        ? cur.doc.path.toLowerCase()
        : null;

    function buildBranch(nodes, depth) {
      const frag = document.createDocumentFragment();
      for (const node of nodes) {
        if (node.is_dir) {
          const dirWrap = document.createElement('div');
          const collapsed = !query && state.collapsedDirs.has(node.path);
          dirWrap.className = 'tree-dir' + (collapsed ? ' collapsed' : '');

          const row = document.createElement('button');
          row.type = 'button';
          row.className = 'tree-row tree-dir-row';
          row.style.paddingLeft = `${depth * 14 + 8}px`;
          row.title = node.path;

          const chevron = document.createElement('span');
          chevron.className = 'tree-chevron';
          chevron.textContent = '▾';

          const label = document.createElement('span');
          label.className = 'tree-label';
          label.textContent = node.name;

          row.appendChild(chevron);
          row.appendChild(label);
          row.addEventListener('click', () => {
            if (state.collapsedDirs.has(node.path)) {
              state.collapsedDirs.delete(node.path);
              dirWrap.classList.remove('collapsed');
            } else {
              state.collapsedDirs.add(node.path);
              dirWrap.classList.add('collapsed');
            }
          });

          const childrenWrap = document.createElement('div');
          childrenWrap.className = 'tree-children';
          childrenWrap.appendChild(buildBranch(node.children || [], depth + 1));

          dirWrap.appendChild(row);
          dirWrap.appendChild(childrenWrap);
          frag.appendChild(dirWrap);
        } else {
          const row = document.createElement('button');
          row.type = 'button';
          const isCurrent = activePath !== null && node.path.toLowerCase() === activePath;
          row.className = 'tree-row' + (isCurrent ? ' active' : '');
          row.style.paddingLeft = `${depth * 14 + 22}px`;
          row.title = node.path;

          const label = document.createElement('span');
          label.className = 'tree-label';
          label.textContent = node.name;

          const dotIdx = node.name.lastIndexOf('.');
          const ext = dotIdx > 0 ? node.name.slice(dotIdx + 1) : '';
          row.appendChild(label);
          if (ext) {
            const badge = document.createElement('span');
            badge.className = 'tree-ext';
            badge.textContent = ext;
            row.appendChild(badge);
          }

          row.addEventListener('click', () => {
            postIpc({ type: 'open_file_path', path: node.path });
          });
          frag.appendChild(row);
        }
      }
      return frag;
    }

    filesTree.appendChild(buildBranch(visibleNodes, 0));
    if (folder.truncated) {
      const note = document.createElement('div');
      note.className = 'tree-truncated';
      note.textContent = 'Showing first 2,000 files';
      filesTree.appendChild(note);
    }
  }

  // ── In-Document Search (Ctrl+F) ────────────────────────────────
  function clearFindHighlights() {
    mdBody.querySelectorAll('mark.mdv-hit').forEach((mark) => {
      const parent = mark.parentNode;
      parent.replaceChild(document.createTextNode(mark.textContent), mark);
      parent.normalize();
    });
    state.hits = [];
    state.hitIdx = -1;
    findCount.textContent = '0 / 0';
  }

  function runFind(query) {
    clearFindHighlights();
    const q = query.trim();
    if (!q) return;

    const lowerQ = q.toLowerCase();
    const walker = document.createTreeWalker(mdBody, NodeFilter.SHOW_TEXT, {
      acceptNode(node) {
        if (!node.nodeValue || !node.nodeValue.trim()) return NodeFilter.FILTER_REJECT;
        if (node.parentElement && node.parentElement.closest('.mermaid, script, style, button')) {
          return NodeFilter.FILTER_REJECT;
        }
        return NodeFilter.FILTER_ACCEPT;
      },
    });

    const textNodes = [];
    let current;
    while ((current = walker.nextNode())) {
      textNodes.push(current);
    }

    for (const node of textNodes) {
      const text = node.nodeValue;
      const lower = text.toLowerCase();
      let idx = lower.indexOf(lowerQ);
      if (idx === -1) continue;

      const frag = document.createDocumentFragment();
      let lastIdx = 0;
      while (idx !== -1) {
        if (idx > lastIdx) {
          frag.appendChild(document.createTextNode(text.slice(lastIdx, idx)));
        }
        const mark = document.createElement('mark');
        mark.className = 'mdv-hit';
        mark.textContent = text.slice(idx, idx + q.length);
        frag.appendChild(mark);
        state.hits.push(mark);
        lastIdx = idx + q.length;
        idx = lower.indexOf(lowerQ, lastIdx);
      }
      if (lastIdx < text.length) {
        frag.appendChild(document.createTextNode(text.slice(lastIdx)));
      }
      node.parentNode.replaceChild(frag, node);
    }

    if (state.hits.length > 0) {
      jumpToHit(0);
    }
  }

  function jumpToHit(index) {
    if (state.hits.length === 0) return;
    if (state.hitIdx >= 0 && state.hits[state.hitIdx]) {
      state.hits[state.hitIdx].classList.remove('active');
    }
    state.hitIdx = (index + state.hits.length) % state.hits.length;
    const hit = state.hits[state.hitIdx];
    hit.classList.add('active');
    hit.scrollIntoView({ block: 'center', behavior: 'smooth' });
    findCount.textContent = `${state.hitIdx + 1} / ${state.hits.length}`;
  }

  function openFindBar() {
    const cur = activeTab();
    if (!cur || cur.kind !== 'doc') return;
    findBar.classList.remove('hidden');
    btnFind.classList.add('active');
    findInput.focus();
    findInput.select();
  }

  function closeFindBar() {
    findBar.classList.add('hidden');
    btnFind.classList.remove('active');
    clearFindHighlights();
  }

  // ── Links Handling (External, Anchors & Relative Docs) ───────────
  function slugifyHeading(text) {
    let out = '';
    let prevDash = false;
    for (const ch of (text || '').trim()) {
      if (/[\p{L}\p{N}]/u.test(ch)) {
        out += ch.toLowerCase();
        prevDash = false;
      } else if ((/\s|-|_/.test(ch)) && out.length > 0 && !prevDash) {
        out += '-';
        prevDash = true;
      }
    }
    return out.replace(/-+$/, '');
  }

  function findHeadingByAnchor(rawAnchor) {
    let decoded = rawAnchor;
    try {
      decoded = decodeURIComponent(rawAnchor);
    } catch (_) {}
    const direct = document.getElementById(decoded) || document.getElementById(rawAnchor);
    if (direct) return direct;
    const targetSlug = slugifyHeading(decoded);
    if (!targetSlug) return null;
    const headings = mdBody.querySelectorAll('h1, h2, h3, h4, h5, h6');
    for (const h of headings) {
      if (h.dataset.slug === targetSlug || slugifyHeading(h.textContent) === targetSlug) {
        return h;
      }
    }
    return null;
  }

  function handleLinkActivation(e) {
    if (e.type === 'auxclick' && e.button !== 1) return;
    const link = e.target.closest && e.target.closest('a');
    if (!link) return;
    e.preventDefault();

    const href = (link.getAttribute('href') || '').trim();
    if (!href) return;
    const lower = href.toLowerCase();

    if (lower.startsWith('http://') || lower.startsWith('https://') || lower.startsWith('mailto:')) {
      postIpc({ type: 'open_external', url: href });
    } else if (lower.startsWith('www.')) {
      postIpc({ type: 'open_external', url: 'https://' + href });
    } else if (href.startsWith('#')) {
      const target = findHeadingByAnchor(href.slice(1));
      if (target) {
        const top = Math.max(0, target.offsetTop - 20);
        readerScroll.scrollTo({ top, behavior: 'smooth' });
      }
    } else {
      const cur = activeTab();
      if (cur && cur.kind === 'doc' && cur.doc && cur.doc.path) {
        postIpc({ type: 'open_relative_link', current_path: cur.doc.path, href });
      }
    }
  }

  mdBody.addEventListener('click', handleLinkActivation);
  mdBody.addEventListener('auxclick', handleLinkActivation);

  // ── UI Button Bindings & Shortcuts ─────────────────────────────
  document.getElementById('btn-new-tab').addEventListener('click', createNewTab);
  if (btnTabsLeft && btnTabsRight) {
    btnTabsLeft.addEventListener('click', () => {
      tabsList.scrollBy({ left: -220, behavior: 'smooth' });
    });
    btnTabsRight.addEventListener('click', () => {
      tabsList.scrollBy({ left: 220, behavior: 'smooth' });
    });
  }
  tabsList.addEventListener(
    'wheel',
    (e) => {
      if (tabsList.scrollWidth > tabsList.clientWidth + 2) {
        e.preventDefault();
        tabsList.scrollLeft += Math.abs(e.deltaX) > Math.abs(e.deltaY) ? e.deltaX : e.deltaY;
        updateTabScrollButtons();
      }
    },
    { passive: false }
  );
  tabsList.addEventListener('scroll', updateTabScrollButtons, { passive: true });
  window.addEventListener('resize', updateTabScrollButtons, { passive: true });
  document.getElementById('btn-welcome-open').addEventListener('click', () => postIpc({ type: 'open_dialog' }));
  document.getElementById('btn-welcome-folder').addEventListener('click', () => postIpc({ type: 'open_folder_dialog' }));
  document.getElementById('btn-welcome-demo').addEventListener('click', () => postIpc({ type: 'open_demo' }));
  btnOpenFolder.addEventListener('click', () => postIpc({ type: 'open_folder_dialog' }));
  btnFilesOpen.addEventListener('click', () => postIpc({ type: 'open_folder_dialog' }));
  btnFilesRefresh.addEventListener('click', () => {
    if (state.folder) postIpc({ type: 'refresh_folder', path: state.folder.root_path });
  });
  filesFilter.addEventListener('input', () => renderFileTree());
  tabOutline.addEventListener('click', () => setSidebarTab('outline'));
  tabFiles.addEventListener('click', () => setSidebarTab('files'));

  btnToc.addEventListener('click', () => {
    state.tocVisible = !state.tocVisible;
    updateSidebarVisibility();
    saveSessionNow();
  });

  btnFind.addEventListener('click', () => {
    if (findBar.classList.contains('hidden')) openFindBar();
    else closeFindBar();
  });
  document.getElementById('find-close').addEventListener('click', closeFindBar);
  document.getElementById('find-next').addEventListener('click', () => jumpToHit(state.hitIdx + 1));
  document.getElementById('find-prev').addEventListener('click', () => jumpToHit(state.hitIdx - 1));
  findInput.addEventListener('input', () => runFind(findInput.value));
  findInput.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      jumpToHit(state.hitIdx + (e.shiftKey ? -1 : 1));
    } else if (e.key === 'Escape') {
      closeFindBar();
    }
  });

  document.getElementById('btn-theme').addEventListener('click', () => applyTheme(!state.dark));
  document.getElementById('btn-zoom-in').addEventListener('click', () => applyZoom(state.zoom + 10));
  document.getElementById('btn-zoom-out').addEventListener('click', () => applyZoom(state.zoom - 10));
  btnZoomReset.addEventListener('click', () => applyZoom(100));
  document.getElementById('btn-print').addEventListener('click', () => window.print());

  document.addEventListener('keydown', (e) => {
    if (!e.ctrlKey && !e.metaKey) return;
    const k = e.key.toLowerCase();
    if (k === 't') {
      e.preventDefault();
      createNewTab();
    } else if (k === 'o' && e.shiftKey) {
      e.preventDefault();
      postIpc({ type: 'open_folder_dialog' });
    } else if (k === 'o') {
      e.preventDefault();
      postIpc({ type: 'open_dialog' });
    } else if (k === 'w') {
      const tab = activeTab();
      if (tab) {
        e.preventDefault();
        closeTab(tab.id);
      }
    } else if (k === 'b') {
      e.preventDefault();
      btnToc.click();
    } else if (k === 'f') {
      e.preventDefault();
      openFindBar();
    } else if (k === 'p') {
      e.preventDefault();
      window.print();
    } else if (k === '=' || k === '+') {
      e.preventDefault();
      applyZoom(state.zoom + 10);
    } else if (k === '-') {
      e.preventDefault();
      applyZoom(state.zoom - 10);
    } else if (k === '0') {
      e.preventDefault();
      applyZoom(100);
    }
  });

  window.addEventListener('beforeunload', saveSessionNow);

  // Notify Rust host that WebView2 DOM is ready to restore session & CLI files
  postIpc({ type: 'ready' });
})();

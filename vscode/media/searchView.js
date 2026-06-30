// searchView.js — webview UI for the loupe sidebar search.
(function () {
  const vscode = acquireVsCodeApi();
  const root = document.body.dataset.root || '';
  const initMax = parseInt(document.body.dataset.max || '300', 10);

  const q = document.getElementById('q');
  const rx = document.getElementById('rx');
  const cs = document.getElementById('cs');
  const toggleFiltersBtn = document.getElementById('toggle-filters');
  const filterRowsEl = document.getElementById('filter-rows');
  const pathFilterEl = document.getElementById('pathFilter');
  const excludeFilterEl = document.getElementById('excludeFilter');
  const maxBtn = document.getElementById('maxBtn');
  const statusEl = document.getElementById('status');
  const resultsEl = document.getElementById('results');

  const MAX_VALUES = [50, 100, 300, 1000, 0]; // 0 = ∞
  const MAX_LABELS = ['50', '100', '300', '1000', '∞'];
  function setMaxLabel(idx) { maxBtn.textContent = 'max ' + MAX_LABELS[idx] + ' ▾'; }

  let maxIdx = (function initMaxIdx() {
    const best = MAX_VALUES.reduce((bi, v, i) => {
      const d = v === 0 ? Infinity : Math.abs(v - initMax);
      const bd = MAX_VALUES[bi] === 0 ? Infinity : Math.abs(MAX_VALUES[bi] - initMax);
      return d < bd ? i : bi;
    }, 0);
    setMaxLabel(best);
    return best;
  })();

  function currentMax() {
    const v = MAX_VALUES[maxIdx];
    return v === 0 ? 999999 : v;
  }

  let regex = false;
  let caseSensitive = false;
  let groups = new Map();
  let fileCount = 0;
  let buffer = [];
  let flushScheduled = false;
  let debounceTimer = null;
  let allResults = []; // all results for current query (used for path-filter re-render without re-search)
  let currentHits = 0;
  let currentQuery = '';
  let pathFilter = '';
  let excludeFilter = '';

  // ---- query dispatch -------------------------------------------------------

  function runSearch() {
    vscode.postMessage({ type: 'search', query: q.value, regex, caseSensitive, max: currentMax() });
  }

  function scheduleSearch() {
    if (debounceTimer) clearTimeout(debounceTimer);
    debounceTimer = setTimeout(runSearch, 250);
  }

  q.addEventListener('input', scheduleSearch);
  q.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') {
      if (debounceTimer) clearTimeout(debounceTimer);
      runSearch();
    }
  });

  rx.addEventListener('click', () => {
    regex = !regex;
    rx.classList.toggle('active', regex);
    runSearch();
  });

  cs.addEventListener('click', () => {
    caseSensitive = !caseSensitive;
    cs.classList.toggle('active', caseSensitive);
    runSearch();
  });

  // Cycle through max values on click, then re-run search.
  maxBtn.addEventListener('click', () => {
    maxIdx = (maxIdx + 1) % MAX_VALUES.length;
    setMaxLabel(maxIdx);
    runSearch();
  });

  // Toggle path filter rows visibility.
  toggleFiltersBtn.addEventListener('click', () => {
    filterRowsEl.toggleAttribute('hidden');
    toggleFiltersBtn.classList.toggle('active', !filterRowsEl.hasAttribute('hidden'));
  });

  // Path filter changes are client-side only — re-render from allResults without a server round-trip.
  pathFilterEl.addEventListener('input', () => {
    pathFilter = pathFilterEl.value;
    rerender();
  });

  excludeFilterEl.addEventListener('input', () => {
    excludeFilter = excludeFilterEl.value;
    rerender();
  });

  // ---- path filter ----------------------------------------------------------

  function pathMatches(pattern, filePath) {
    if (!pattern) return true;
    const pat = pattern.trim();
    if (!pat) return true;
    if (!pat.includes('*') && !pat.includes('?')) {
      return filePath.toLowerCase().includes(pat.toLowerCase());
    }
    return globTest(pat, filePath);
  }

  // Minimal glob → regex: * = any non-separator, ** = any, ? = single non-separator.
  function globTest(pattern, text) {
    let re = '';
    let i = 0;
    while (i < pattern.length) {
      const ch = pattern[i];
      if (ch === '*' && pattern[i + 1] === '*') {
        re += '.*';
        i += 2;
        if (pattern[i] === '/') i++;
      } else if (ch === '*') {
        re += '[^/]*';
        i++;
      } else if (ch === '?') {
        re += '[^/]';
        i++;
      } else {
        re += ch.replace(/[.+^${}()|[\]\\]/g, '\\$&');
        i++;
      }
    }
    try {
      return new RegExp(re, 'i').test(text);
    } catch {
      return true;
    }
  }

  // ---- result rendering -----------------------------------------------------

  function reset() {
    groups = new Map();
    fileCount = 0;
    buffer = [];
    resultsEl.textContent = '';
  }

  function rerender() {
    reset();
    for (const r of allResults) {
      addRow(r.file, r.line, r.text);
    }
    updateStatusDone();
  }

  function updateStatusDone() {
    if (!currentQuery) {
      statusEl.textContent = '';
      return;
    }
    if (currentHits === 0) {
      statusEl.textContent = 'No results';
      return;
    }
    const suffix = (pathFilter || excludeFilter) ? ' (path filtered)' : '';
    statusEl.textContent =
      currentHits + ' result' + (currentHits === 1 ? '' : 's') +
      ' in ' + fileCount + ' file' + (fileCount === 1 ? '' : 's') +
      suffix;
  }

  function relPath(file) {
    if (file === root) return file;
    if (root && file.startsWith(root + '/')) return file.slice(root.length + 1);
    return file;
  }

  function basename(p) {
    const i = p.lastIndexOf('/');
    return i >= 0 ? p.slice(i + 1) : p;
  }

  function dirname(p) {
    const i = p.lastIndexOf('/');
    return i >= 0 ? p.slice(0, i) : '';
  }

  function escapeHtml(s) {
    return s.replace(/[&<>"]/g, (c) =>
      c === '&' ? '&amp;' : c === '<' ? '&lt;' : c === '>' ? '&gt;' : '&quot;'
    );
  }

  function highlight(text, query) {
    const ranges = matchRanges(text, query);
    if (!ranges.length) return { html: escapeHtml(text), column: 0 };
    let html = '';
    let pos = 0;
    for (const [start, end] of ranges) {
      html += escapeHtml(text.slice(pos, start));
      html += '<span class="hl">' + escapeHtml(text.slice(start, end)) + '</span>';
      pos = end;
    }
    html += escapeHtml(text.slice(pos));
    return { html, column: ranges[0][0] };
  }

  function matchRanges(text, query) {
    const ranges = [];
    if (!query) return ranges;
    if (regex) {
      let re;
      try {
        re = new RegExp(query, caseSensitive ? 'g' : 'gi');
      } catch {
        return ranges;
      }
      let m;
      while ((m = re.exec(text)) !== null) {
        if (m[0] === '') { re.lastIndex++; continue; }
        ranges.push([m.index, m.index + m[0].length]);
        if (ranges.length > 200) break;
      }
    } else {
      const hay = caseSensitive ? text : text.toLowerCase();
      const needle = caseSensitive ? query : query.toLowerCase();
      let from = 0, idx;
      while ((idx = hay.indexOf(needle, from)) !== -1) {
        ranges.push([idx, idx + needle.length]);
        from = idx + needle.length;
        if (ranges.length > 200) break;
      }
    }
    return ranges;
  }

  function groupFor(file) {
    let g = groups.get(file);
    if (g) return g;

    const groupEl = document.createElement('div');
    groupEl.className = 'group';

    const header = document.createElement('div');
    header.className = 'group-header';

    const rel = relPath(file);

    // Line 1: twisty + filename
    const line1 = document.createElement('div');
    line1.className = 'group-header-line1';
    const twistyEl = document.createElement('span');
    twistyEl.className = 'twisty';
    twistyEl.textContent = '▾';
    const fnameEl = document.createElement('span');
    fnameEl.className = 'fname';
    fnameEl.textContent = basename(rel);
    line1.appendChild(twistyEl);
    line1.appendChild(fnameEl);

    // Line 2: directory + count
    const line2 = document.createElement('div');
    line2.className = 'group-header-line2';
    const fdirEl = document.createElement('span');
    fdirEl.className = 'fdir';
    fdirEl.textContent = dirname(rel);
    const fcountEl = document.createElement('span');
    fcountEl.className = 'fcount';
    fcountEl.textContent = '0';
    line2.appendChild(fdirEl);
    line2.appendChild(fcountEl);

    header.title = rel; // full path on hover
    header.appendChild(line1);
    header.appendChild(line2);
    header.addEventListener('click', () => groupEl.classList.toggle('collapsed'));

    const rowsEl = document.createElement('div');
    rowsEl.className = 'rows';

    groupEl.appendChild(header);
    groupEl.appendChild(rowsEl);
    resultsEl.appendChild(groupEl);

    g = { rowsEl, groupEl, count: 0, countEl: fcountEl };
    groups.set(file, g);
    fileCount++;
    return g;
  }

  function addRow(file, line, text) {
    const rel = relPath(file);
    if (pathFilter && !pathMatches(pathFilter, rel)) return;
    if (excludeFilter && pathMatches(excludeFilter, rel)) return; // exclude: matches → skip
    const g = groupFor(file);
    const { html, column } = highlight(text, q.value);
    const row = document.createElement('div');
    row.className = 'row';
    row.dataset.file = file;
    row.dataset.line = String(line);
    row.dataset.column = String(column);
    row.innerHTML = '<span class="ln">' + line + '</span><span class="txt">' + html + '</span>';
    g.rowsEl.appendChild(row);
    g.count++;
    g.countEl.textContent = String(g.count);
  }

  function flush() {
    flushScheduled = false;
    const batch = buffer;
    buffer = [];
    for (const r of batch) {
      addRow(r.file, r.line, r.text);
    }
  }

  function scheduleFlush() {
    if (!flushScheduled) {
      flushScheduled = true;
      requestAnimationFrame(flush);
    }
  }

  resultsEl.addEventListener('click', (e) => {
    const row = e.target.closest('.row');
    if (row) {
      vscode.postMessage({
        type: 'open',
        file: row.dataset.file,
        line: Number(row.dataset.line),
        column: Number(row.dataset.column),
      });
    }
  });

  // ---- messages from the extension -----------------------------------------

  window.addEventListener('message', (e) => {
    const msg = e.data;
    if (!msg) return;
    switch (msg.type) {
      case 'begin':
        allResults = [];
        currentHits = 0;
        currentQuery = msg.query || '';
        reset();
        statusEl.textContent = msg.query ? 'Searching…' : '';
        statusEl.classList.remove('error');
        break;
      case 'result':
        allResults.push(msg);
        buffer.push(msg);
        scheduleFlush();
        break;
      case 'done':
        flush();
        currentHits = msg.hits || 0;
        if (msg.tooShort) {
          statusEl.textContent = 'Type at least 3 characters';
        } else {
          updateStatusDone();
        }
        break;
      case 'error':
        statusEl.textContent = 'Error: ' + (msg.message || 'unknown');
        statusEl.classList.add('error');
        break;
      case 'focus':
        q.focus();
        q.select();
        break;
    }
  });

  q.focus();
})();

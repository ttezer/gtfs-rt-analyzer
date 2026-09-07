import init, { analyze_feed } from "./pkg/gtfs_rt_wasm.js";

const $ = (id) => document.getElementById(id);
const state = {
  ready: false,
  timer: null,
  inFlight: false,
  history: [],
  lastStartedAt: null,
};

const catalogState = {
  feeds: [],
  markers: [],
  map: null,
  scoreCache: new Map(),
  scoreRequest: 0,
  analyzerModule: null,
};

const elements = {
  wasmStatus: $("wasm-status"),
  monitorState: $("monitor-state"),
  feedUrl: $("feed-url"),
  proxyUrl: $("proxy-url"),
  interval: $("interval"),
  fetchNow: $("fetch-now"),
  startMonitoring: $("start-monitoring"),
  stopMonitoring: $("stop-monitoring"),
  requestStatus: $("request-status"),
  fileInput: $("file-input"),
  analyzeFile: $("analyze-file"),
  reportTitle: $("report-title"),
  sourceBadge: $("source-badge"),
  feedAge: $("feed-age"),
  entityCount: $("entity-count"),
  anomalyCount: $("anomaly-count"),
  lastSuccess: $("last-success"),
  entityBreakdown: $("entity-breakdown"),
  reportStatus: $("report-status"),
  anomalies: $("anomalies"),
  history: $("history"),
  catalogSearch: $("catalog-search"),
  catalogType: $("catalog-type"),
  catalogStatus: $("catalog-feed-status"),
  catalogProvenance: $("catalog-provenance"),
  catalogCount: $("catalog-count"),
  catalogNotice: $("catalog-status"),
  scheduleScore: $("schedule-score"),
  feedMap: $("feed-map"),
  feedList: $("feed-list"),
};

function setNotice(element, text, kind = "neutral") {
  element.className = `notice ${kind}`;
  element.textContent = text;
}

function setWasmStatus(text, kind) {
  elements.wasmStatus.className = `status-pill ${kind}`;
  elements.wasmStatus.textContent = text;
}

function formatClock(timestamp) {
  if (!timestamp) return "—";
  return new Intl.DateTimeFormat("tr-TR", {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  }).format(new Date(timestamp));
}

function formatAge(seconds) {
  if (seconds === null || seconds === undefined) return "—";
  const age = Math.round(Date.now() / 1000 - Number(seconds));
  if (!Number.isFinite(age)) return "—";
  if (age < -5) return `${Math.abs(age)} sn ileride`;
  if (age < 60) return `${Math.max(0, age)} sn`;
  if (age < 3600) return `${Math.floor(age / 60)} dk`;
  return `${Math.floor(age / 3600)} sa`;
}

const TYPE_LABELS = {
  vehicle_positions: "Araç",
  trip_updates: "Sefer",
  alerts: "Uyarı",
};

function escapeHtml(value) {
  return String(value ?? "")
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#039;");
}

function feedTitle(feed) {
  return feed.name ? `${feed.provider} — ${feed.name}` : feed.provider;
}

function feedTypeLabels(feed) {
  return (feed.entity_types || []).map((type) => TYPE_LABELS[type] || type);
}

function feedProvenance(feed) {
  if (feed.official === true) return "official";
  if (feed.official === false) return "community";
  return "unknown";
}

function feedCenter(feed) {
  if (!feed.center) return null;
  const lat = Number(feed.center.lat);
  const lon = Number(feed.center.lon);
  return Number.isFinite(lat) && Number.isFinite(lon) ? [lat, lon] : null;
}

function scheduleUrl(feed) {
  return feed.static_reference
    ? `https://files.mobilitydatabase.org/${encodeURIComponent(feed.static_reference)}/latest.zip`
    : null;
}

function todayNumber() {
  const now = new Date();
  return now.getFullYear() * 10000 + (now.getMonth() + 1) * 100 + now.getDate();
}

function setScheduleNotice(text, kind = "neutral") {
  setNotice(elements.scheduleScore, text, kind);
}

function showScheduleOption(feed) {
  const cached = feed.static_reference && catalogState.scoreCache.get(feed.static_reference);
  if (cached) {
    renderScheduleResult(feed, cached);
    return;
  }
  if (!feed.static_reference) {
    setScheduleNotice("Bu feed için katalogda bağlı Schedule yok; skor hesaplanamaz.", "neutral");
    return;
  }

  elements.scheduleScore.className = "notice neutral";
  elements.scheduleScore.innerHTML = "";
  const title = document.createElement("strong");
  title.textContent = `Bağlı Schedule · ${feed.static_reference}`;
  elements.scheduleScore.append(title);
  const description = document.createElement("div");
  description.textContent = "Yayın ve Genel skoru, Schedule ZIP'i indirildikten sonra tarayıcıda hesaplanır.";
  elements.scheduleScore.append(description);
  const button = document.createElement("button");
  button.type = "button";
  button.textContent = "Schedule skorunu hesapla";
  button.addEventListener("click", () => void loadScheduleScore(feed));
  elements.scheduleScore.append(button);
}

function renderScheduleResult(feed, result) {
  const r5 = result?.reports?.r5;
  if (!r5) {
    setScheduleNotice("Schedule çözümlendi ancak skor raporu bulunamadı.", "warning");
    return;
  }

  elements.scheduleScore.className = "notice success";
  elements.scheduleScore.innerHTML = "";
  const title = document.createElement("strong");
  title.textContent = `Bağlı Schedule · ${feed.static_reference}`;
  elements.scheduleScore.append(title);

  const values = document.createElement("div");
  values.className = "schedule-score-values";
  for (const [label, value] of [["Yayın", r5.pub_score], ["Genel", r5.score]]) {
    const item = document.createElement("span");
    item.className = "schedule-score-value";
    item.append(`${label} `);
    const score = document.createElement("strong");
    score.textContent = `${Number(value).toFixed(1)}/100`;
    item.append(score);
    values.append(item);
  }
  elements.scheduleScore.append(values);

  if (result.validation_status === "PARTIAL") {
    const partial = document.createElement("div");
    partial.className = "muted";
    partial.textContent = "Analyzer doğrulaması eksik kapsamlı; skor temkinli yorumlanmalı.";
    elements.scheduleScore.append(partial);
  }
}

async function loadScheduleScore(feed) {
  const requestId = ++catalogState.scoreRequest;
  if (!feed.static_reference) {
    setScheduleNotice("Bu feed için katalogda bağlı Schedule yok; skor hesaplanamaz.", "neutral");
    return;
  }

  const cached = catalogState.scoreCache.get(feed.static_reference);
  if (cached) {
    renderScheduleResult(feed, cached);
    return;
  }

  const url = scheduleUrl(feed);
  setScheduleNotice("Bağlı Schedule indiriliyor ve analyzer çalışıyor…", "neutral");
  const controller = new AbortController();
  const timeout = window.setTimeout(() => controller.abort(), 90_000);

  try {
    const response = await fetch(url, { cache: "no-store", signal: controller.signal });
    if (!response.ok) throw new Error(`${response.status} ${response.statusText}`);
    const bytes = new Uint8Array(await response.arrayBuffer());
    if (requestId !== catalogState.scoreRequest) return;

    if (!catalogState.analyzerModule) {
      const module = await import("./pkg/schedule/gtfs_wasm.js");
      await module.default();
      catalogState.analyzerModule = module;
    }
    const raw = catalogState.analyzerModule.validate_with_today(bytes, "{}", todayNumber());
    if (!raw?.Ok) {
      throw new Error(raw?.Fatal?.message || "Schedule doğrulanamadı");
    }
    catalogState.scoreCache.set(feed.static_reference, raw.Ok);
    if (requestId === catalogState.scoreRequest) renderScheduleResult(feed, raw.Ok);
  } catch (error) {
    if (requestId !== catalogState.scoreRequest) return;
    const message = error?.name === "AbortError"
      ? "Schedule isteği zaman aşımına uğradı."
      : error?.message || "Schedule skoru alınamadı.";
    setScheduleNotice(`Schedule skoru alınamadı: ${message}`, "warning");
  } finally {
    window.clearTimeout(timeout);
  }
}

function feedMatches(feed) {
  const search = elements.catalogSearch.value.trim().toLocaleLowerCase("tr-TR");
  const haystack = [feed.provider, feed.name, feed.municipality, feed.country_code]
    .filter(Boolean)
    .join(" ")
    .toLocaleLowerCase("tr-TR");
  const searchMatches = !search || haystack.includes(search);
  const typeMatches = elements.catalogType.value === "all"
    || feed.entity_types?.includes(elements.catalogType.value);
  const statusMatches = elements.catalogStatus.value === "all"
    || feed.status === elements.catalogStatus.value;
  const provenanceMatches = elements.catalogProvenance.value === "all"
    || feedProvenance(feed) === elements.catalogProvenance.value;
  return searchMatches && typeMatches && statusMatches && provenanceMatches;
}

function selectCatalogFeed(feed) {
  elements.feedUrl.value = feed.url;
  if (!elements.proxyUrl.value.trim()) {
    elements.proxyUrl.value = "https://gtfs-rt-proxy.ttezer.workers.dev";
  }
  setNotice(
    elements.requestStatus,
    `${feedTitle(feed)} seçildi. Şimdi çek düğmesine basın.`,
    "neutral",
  );
  elements.feedUrl.focus();
  showScheduleOption(feed);
}

function popupForFeed(feed) {
  const types = feedTypeLabels(feed).join(", ") || "Tür belirtilmemiş";
  const provenance = feed.official === true
    ? "Resmî"
    : feed.official === false
      ? "Topluluk"
      : "Belirsiz";
  const schedule = feed.static_reference ? "Schedule bağlantısı var" : "Schedule bağlantısı yok";
  return `<strong>${escapeHtml(feedTitle(feed))}</strong><br>${escapeHtml(types)} · ${escapeHtml(provenance)}<br>${escapeHtml(schedule)}`;
}

function clearMapMarkers() {
  for (const marker of catalogState.markers) marker.remove();
  catalogState.markers = [];
}

function renderCatalog() {
  const filtered = catalogState.feeds.filter(feedMatches);
  elements.catalogCount.textContent = `${filtered.length.toLocaleString("tr-TR")} feed`;
  elements.catalogNotice.textContent = filtered.length
    ? "Bir feed seçin; URL izleme formuna doldurulur."
    : "Bu filtrelerle feed bulunamadı.";
  elements.catalogNotice.className = "notice neutral";

  if (catalogState.map) {
    clearMapMarkers();
    const markerGroup = [];
    for (const feed of filtered) {
      const center = feedCenter(feed);
      if (center === null) continue;
      const color = feed.official === true ? "#176b62" : feed.official === false ? "#8a641d" : "#536473";
      const marker = window.L.circleMarker(center, {
        radius: 5,
        color,
        weight: 1,
        fillColor: color,
        fillOpacity: 0.75,
      });
      marker.bindPopup(popupForFeed(feed));
      marker.on("click", () => selectCatalogFeed(feed));
      marker.addTo(catalogState.map);
      catalogState.markers.push(marker);
      markerGroup.push(center);
    }
    if (markerGroup.length > 0 && filtered.length < 30) {
      catalogState.map.fitBounds(markerGroup, { padding: [20, 20], maxZoom: 10 });
    }
  }

  elements.feedList.innerHTML = "";
  const fragment = document.createDocumentFragment();
  for (const feed of filtered) {
    const card = document.createElement("article");
    card.className = "feed-card";

    const header = document.createElement("div");
    header.className = "feed-card-header";
    const title = document.createElement("h3");
    title.textContent = feedTitle(feed);
    header.append(title);
    const selectButton = document.createElement("button");
    selectButton.type = "button";
    selectButton.textContent = "Seç";
    selectButton.addEventListener("click", () => selectCatalogFeed(feed));
    header.append(selectButton);
    card.append(header);

    const location = [feed.municipality, feed.country_code].filter(Boolean).join(", ");
    if (location) {
      const locationText = document.createElement("p");
      locationText.textContent = location;
      card.append(locationText);
    }

    const meta = document.createElement("div");
    meta.className = "feed-meta";
    for (const label of feedTypeLabels(feed)) {
      const tag = document.createElement("span");
      tag.className = "feed-tag";
      tag.textContent = label;
      meta.append(tag);
    }
    const provenanceTag = document.createElement("span");
    provenanceTag.className = `feed-tag ${feedProvenance(feed)}`;
    provenanceTag.textContent = feed.official === true
      ? "Resmî"
      : feed.official === false
        ? "Topluluk"
        : "Belirsiz";
    meta.append(provenanceTag);
    const statusTag = document.createElement("span");
    statusTag.className = "feed-tag";
    statusTag.textContent = feed.status;
    meta.append(statusTag);
    const scheduleTag = document.createElement("span");
    scheduleTag.className = "feed-tag";
    scheduleTag.textContent = feed.static_reference ? "Schedule bağlı" : "Schedule yok";
    meta.append(scheduleTag);
    card.append(meta);
    fragment.append(card);
  }
  elements.feedList.append(fragment);
}

async function initCatalog() {
  if (window.L) {
    catalogState.map = window.L.map(elements.feedMap, { worldCopyJump: true }).setView([25, 10], 2);
    window.L.tileLayer("https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png", {
      attribution: "&copy; OpenStreetMap contributors",
      maxZoom: 18,
    }).addTo(catalogState.map);
  } else {
    elements.feedMap.textContent = "Harita kütüphanesi yüklenemedi; liste kullanılabilir.";
  }

  try {
    const response = await fetch("./feeds.json", { cache: "no-store" });
    if (!response.ok) throw new Error(`${response.status} ${response.statusText}`);
    const payload = await response.json();
    catalogState.feeds = Array.isArray(payload.feeds) ? payload.feeds : [];
    renderCatalog();
  } catch (error) {
    elements.catalogCount.textContent = "Katalog yok";
    elements.catalogNotice.className = "notice error";
    elements.catalogNotice.textContent = `Katalog yüklenemedi: ${error?.message || "bilinmeyen hata"}`;
  }
}

function proxyFetchUrl(proxyBase, feedUrl) {
  const base = new URL(proxyBase);
  if (base.pathname === "/" || base.pathname === "") base.pathname = "/fetch";
  base.searchParams.set("url", feedUrl);
  return base.toString();
}

async function fetchBytes(url) {
  const controller = new AbortController();
  const timer = window.setTimeout(() => controller.abort(), 20_000);

  try {
    const response = await fetch(url, {
      cache: "no-store",
      redirect: "follow",
      signal: controller.signal,
    });

    const bytes = new Uint8Array(await response.arrayBuffer());
    if (!response.ok) {
      const proxyError = response.headers.get("X-Proxy-Error");
      const detail = proxyError || `${response.status} ${response.statusText}`;
      throw Object.assign(new Error(detail), { kind: "upstream_error" });
    }

    return bytes;
  } catch (error) {
    if (error?.name === "AbortError") {
      throw Object.assign(new Error("İstek zaman aşımına uğradı"), { kind: "timeout" });
    }
    if (error?.kind) throw error;
    if (error instanceof TypeError) {
      throw Object.assign(new Error("Tarayıcı doğrudan erişemedi; CORS veya ağ hatası"), {
        kind: "cors_blocked",
      });
    }
    throw Object.assign(new Error(error?.message || "İstek başarısız"), {
      kind: "upstream_error",
    });
  } finally {
    window.clearTimeout(timer);
  }
}

function reportBytes(bytes) {
  return JSON.parse(analyze_feed(bytes));
}

function reportSummary(report, source, completedAt, actualInterval) {
  return {
    source,
    completedAt,
    actualInterval,
    bytes: report.bytes,
    entities: report.entities.total,
    anomalies: report.anomalies.length,
    feedTimestamp: report.header?.timestamp ?? null,
  };
}

function renderReport(report, source, completedAt, actualInterval) {
  const header = report.header;
  elements.reportTitle.textContent = header?.gtfs_realtime_version
    ? `GTFS-RT ${header.gtfs_realtime_version}`
    : "Feed raporu";
  elements.sourceBadge.className = `badge ${source}`;
  elements.sourceBadge.textContent = source === "file" ? "dosya" : source;
  elements.feedAge.textContent = formatAge(header?.timestamp);
  elements.entityCount.textContent = String(report.entities.total);
  elements.anomalyCount.textContent = String(report.anomalies.length);
  elements.lastSuccess.textContent = formatClock(completedAt);
  elements.entityBreakdown.innerHTML = "";

  const breakdown = [
    ["TripUpdate", report.entities.trip_updates],
    ["Vehicle", report.entities.vehicles],
    ["Alert", report.entities.alerts],
    ["Payload yok", report.entities.without_payload],
  ];
  for (const [label, count] of breakdown) {
    const item = document.createElement("span");
    item.textContent = `${label} ${count}`;
    elements.entityBreakdown.append(item);
  }

  const status = report.anomalies.length === 0
    ? `Temiz snapshot · ${report.bytes} bayt · ${formatClock(completedAt)}`
    : `${report.anomalies.length} anomali · ${report.bytes} bayt · ${formatClock(completedAt)}`;
  setNotice(elements.reportStatus, status, report.anomalies.length === 0 ? "success" : "warning");

  elements.anomalies.innerHTML = "";
  elements.anomalies.className = report.anomalies.length === 0 ? "anomaly-list empty" : "anomaly-list";
  if (report.anomalies.length === 0) {
    elements.anomalies.textContent = "Anomali yok.";
  } else {
    for (const anomaly of report.anomalies.slice(0, 80)) {
      const item = document.createElement("div");
      item.className = "anomaly";
      item.innerHTML = `<div class="anomaly-top"><span class="anomaly-kind"></span><span>@${anomaly.offset} · ${anomaly.level}</span></div><div class="anomaly-path"></div>`;
      item.querySelector(".anomaly-kind").textContent = anomaly.kind;
      item.querySelector(".anomaly-path").textContent = anomaly.path || anomaly.message;
      elements.anomalies.append(item);
    }
    if (report.anomalies.length > 80) {
      const more = document.createElement("div");
      more.className = "muted";
      more.textContent = `${report.anomalies.length - 80} anomali daha var.`;
      elements.anomalies.append(more);
    }
  }

  state.history.unshift(reportSummary(report, source, completedAt, actualInterval));
  state.history = state.history.slice(0, 8);
  renderHistory();
}

function renderHistory() {
  elements.history.innerHTML = "";
  elements.history.className = state.history.length ? "history" : "history empty";
  if (!state.history.length) {
    elements.history.textContent = "Henüz çekim yok.";
    return;
  }

  for (const item of state.history) {
    const row = document.createElement("div");
    row.className = "history-row";
    const interval = item.actualInterval === null ? "ilk çekim" : `gerçekleşen ${item.actualInterval} sn`;
    row.innerHTML = `<span><strong>${item.source}</strong> · ${item.entities} entity · ${item.anomalies} anomali</span><small>${formatClock(item.completedAt)} · ${interval}</small>`;
    elements.history.append(row);
  }
}

async function requestFeed() {
  if (!state.ready || state.inFlight) return;
  const feedUrl = elements.feedUrl.value.trim();
  if (!feedUrl) {
    setNotice(elements.requestStatus, "Önce bir GTFS-RT URL girin.", "warning");
    return;
  }

  state.inFlight = true;
  elements.fetchNow.disabled = true;
  elements.startMonitoring.disabled = true;
  const startedAt = Date.now();
  setNotice(elements.requestStatus, "Doğrudan feed deneniyor…", "neutral");

  try {
    let bytes;
    let source = "direct";
    try {
      bytes = await fetchBytes(feedUrl);
    } catch (directError) {
      if (directError.kind !== "cors_blocked" || !elements.proxyUrl.value.trim()) {
        throw directError;
      }
      source = "proxy";
      setNotice(elements.requestStatus, "Direct erişim CORS nedeniyle başarısız; proxy deneniyor…", "warning");
      bytes = await fetchBytes(proxyFetchUrl(elements.proxyUrl.value.trim(), feedUrl));
    }

    const completedAt = Date.now();
    const actualInterval = state.lastStartedAt === null
      ? null
      : Math.round((startedAt - state.lastStartedAt) / 1000);
    state.lastStartedAt = startedAt;
    const report = reportBytes(bytes);
    renderReport(report, source, completedAt, actualInterval);
    setNotice(elements.requestStatus, `${source === "direct" ? "Direct" : "Proxy"} başarılı · ${bytes.byteLength} bayt`, "success");
  } catch (error) {
    const kind = error?.kind || "upstream_error";
    setNotice(elements.requestStatus, `${kind}: ${error?.message || "İstek başarısız"}`, kind === "timeout" ? "warning" : "error");
  } finally {
    state.inFlight = false;
    elements.fetchNow.disabled = false;
    elements.startMonitoring.disabled = false;
  }
}

function startMonitoring() {
  if (!state.ready || state.timer !== null) return;
  state.lastStartedAt = null;
  elements.monitorState.textContent = "İzleniyor";
  elements.stopMonitoring.disabled = false;
  requestFeed();
  const intervalMs = Number(elements.interval.value) * 1000;
  state.timer = window.setInterval(requestFeed, intervalMs);
  setNotice(elements.requestStatus, `İzleme başladı · hedef aralık ${intervalMs / 1000} saniye`, "neutral");
}

function stopMonitoring() {
  if (state.timer !== null) window.clearInterval(state.timer);
  state.timer = null;
  elements.monitorState.textContent = "Durduruldu";
  elements.stopMonitoring.disabled = true;
  setNotice(elements.requestStatus, "İzleme durduruldu.", "neutral");
}

async function analyzeFile() {
  const file = elements.fileInput.files?.[0];
  if (!file || !state.ready) return;
  try {
    const bytes = new Uint8Array(await file.arrayBuffer());
    const report = reportBytes(bytes);
    const completedAt = Date.now();
    renderReport(report, "file", completedAt, null);
    setNotice(elements.requestStatus, `Dosya analiz edildi · ${file.name} · ${bytes.byteLength} bayt`, "success");
  } catch (error) {
    setNotice(elements.requestStatus, `Dosya analizi başarısız: ${error?.message || "bilinmeyen hata"}`, "error");
  }
}

elements.fetchNow.addEventListener("click", requestFeed);
elements.startMonitoring.addEventListener("click", startMonitoring);
elements.stopMonitoring.addEventListener("click", stopMonitoring);
elements.fileInput.addEventListener("change", () => {
  elements.analyzeFile.disabled = !elements.fileInput.files?.length || !state.ready;
});
elements.analyzeFile.addEventListener("click", analyzeFile);
elements.catalogSearch.addEventListener("input", renderCatalog);
elements.catalogType.addEventListener("change", renderCatalog);
elements.catalogStatus.addEventListener("change", renderCatalog);
elements.catalogProvenance.addEventListener("change", renderCatalog);

void initCatalog();

try {
  await init();
  state.ready = true;
  elements.analyzeFile.disabled = !elements.fileInput.files?.length;
  setWasmStatus("WASM hazır", "ready");
} catch (error) {
  setWasmStatus("WASM yüklenemedi", "error");
  setNotice(elements.requestStatus, `WASM başlatılamadı: ${error?.message || "bilinmeyen hata"}`, "error");
}

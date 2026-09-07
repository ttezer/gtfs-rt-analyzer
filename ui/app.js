import init, { analyze_feed } from "./pkg/gtfs_rt_wasm.js";

const $ = (id) => document.getElementById(id);
const state = {
  ready: false,
  timer: null,
  inFlight: false,
  history: [],
  lastStartedAt: null,
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

try {
  await init();
  state.ready = true;
  elements.analyzeFile.disabled = !elements.fileInput.files?.length;
  setWasmStatus("WASM hazır", "ready");
} catch (error) {
  setWasmStatus("WASM yüklenemedi", "error");
  setNotice(elements.requestStatus, `WASM başlatılamadı: ${error?.message || "bilinmeyen hata"}`, "error");
}

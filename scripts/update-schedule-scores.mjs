import { createHash } from "node:crypto";
import { readFile, writeFile } from "node:fs/promises";
import { getVersion, validateGtfs } from "gtfs-sdk";

const catalogPath = new URL("../ui/feeds.json", import.meta.url);
const scoresPath = new URL("../ui/schedule-scores.json", import.meta.url);
const catalog = JSON.parse(await readFile(catalogPath, "utf8"));
const previous = await readJson(scoresPath, {
  source: "gtfs-analyzer",
  scores: [],
});
const previousByReference = new Map(
  (previous.scores || []).map((score) => [score.static_reference, score]),
);
const references = [...new Set(
  (catalog.feeds || []).map((feed) => feed.static_reference).filter(Boolean),
)].sort();
const today = process.env.SCHEDULE_SCORE_DATE || new Date().toISOString().slice(0, 10);
const limit = Number(process.env.SCHEDULE_SCORE_LIMIT || 0);
const selectedReferences = limit > 0 ? references.slice(0, limit) : references;
const scores = [];

console.log(`Refreshing ${selectedReferences.length} of ${references.length} Schedule references`);
console.log(`gtfs-sdk ${getVersion().sdk}, engine ${getVersion().engine}, today ${today}`);

for (const [index, staticReference] of selectedReferences.entries()) {
  const scheduleUrl = `https://files.mobilitydatabase.org/${encodeURIComponent(staticReference)}/latest.zip`;
  const old = previousByReference.get(staticReference);
  let metadata;

  try {
    metadata = await readRemoteMetadata(scheduleUrl);
    if (old && sameSnapshot(old, metadata)) {
      scores.push(old);
      console.log(`[${index + 1}/${selectedReferences.length}] ${staticReference}: unchanged`);
      continue;
    }

    const response = await fetch(scheduleUrl, { redirect: "follow" });
    if (!response.ok) throw new Error(`${response.status} ${response.statusText}`);
    const bytes = new Uint8Array(await response.arrayBuffer());
    const hash = createHash("sha256").update(bytes).digest("hex");
    const result = await validateGtfs(bytes, { today });
    const score = result.reports?.r5;
    if (!score) throw new Error("Analyzer response did not contain an R5 score");

    scores.push({
      static_reference: staticReference,
      schedule_url: scheduleUrl,
      etag: metadata.etag,
      last_modified: metadata.last_modified,
      content_length: metadata.content_length || bytes.byteLength,
      sha256: hash,
      publish_score: score.pub_score,
      overall_score: score.score,
      validation_status: result.validation_status || "COMPLETE",
      analyzed_at: new Date().toISOString(),
    });
    console.log(`[${index + 1}/${selectedReferences.length}] ${staticReference}: ${score.pub_score.toFixed(1)} / ${score.score.toFixed(1)}`);
  } catch (error) {
    scores.push({
      static_reference: staticReference,
      schedule_url: scheduleUrl,
      etag: metadata?.etag || null,
      last_modified: metadata?.last_modified || null,
      content_length: metadata?.content_length || null,
      publish_score: null,
      overall_score: null,
      validation_status: "ERROR",
      error: error?.message || String(error),
      analyzed_at: new Date().toISOString(),
    });
    console.error(`[${index + 1}/${selectedReferences.length}] ${staticReference}: ${error?.message || error}`);
  }
}

for (const score of previous.scores || []) {
  if (!scores.some((item) => item.static_reference === score.static_reference)) {
    scores.push(score);
  }
}
scores.sort((a, b) => a.static_reference.localeCompare(b.static_reference));

const scoresChanged = JSON.stringify(previous.scores || []) !== JSON.stringify(scores);

const output = {
  source: "gtfs-analyzer",
  source_url: "https://github.com/ttezer/gtfs-analyzer",
  engine_version: getVersion().engine,
  updated_at: scoresChanged ? new Date().toISOString().slice(0, 10) : (previous.updated_at || today),
  scores,
};
const serialized = `${JSON.stringify(output, null, 2)}\n`;
const oldSerialized = await readJsonText(scoresPath);
if (oldSerialized !== serialized) {
  await writeFile(scoresPath, serialized);
  console.log(`Wrote ${scores.length} score records to ${scoresPath.pathname}`);
} else {
  console.log("No score snapshot changes");
}

async function readRemoteMetadata(url) {
  const response = await fetch(url, { method: "HEAD", redirect: "follow" });
  if (!response.ok && ![403, 405].includes(response.status)) {
    throw new Error(`HEAD ${response.status} ${response.statusText}`);
  }
  if (!response.ok) return { etag: null, last_modified: null, content_length: null };
  return {
    etag: response.headers.get("etag"),
    last_modified: response.headers.get("last-modified"),
    content_length: response.headers.get("content-length")
      ? Number(response.headers.get("content-length"))
      : null,
  };
}

function sameSnapshot(old, current) {
  if (old.validation_status === "ERROR" || old.publish_score === null || old.overall_score === null) {
    return false;
  }
  if (old.etag && current.etag) return old.etag === current.etag;
  return Boolean(
    old.last_modified && current.last_modified
      && old.last_modified === current.last_modified
      && old.content_length === current.content_length,
  );
}

async function readJson(url, fallback) {
  try {
    return JSON.parse(await readFile(url, "utf8"));
  } catch {
    return fallback;
  }
}

async function readJsonText(url) {
  try {
    return await readFile(url, "utf8");
  } catch {
    return null;
  }
}
